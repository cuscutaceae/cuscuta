use std::{collections::HashMap, time::Duration};

use chrono::Utc;
use cuscuta_common::{
    api::{
        self,
        xxxxxx::{
            FriendDelta, FriendInfo, SongScore, api_delete_friend, auto::xxxxxx_safe_call,
            calc_friend_delta,
        },
    },
    data::{BundleData, Song},
    db::{
        self,
        account::AccountRow,
        job::{
            Job, JobState, JobTag, SubQueue,
            enqueue::{write_job, write_job_index},
            eta::record_eta,
            scan_sub_queue,
        },
        redis::{job_pending_index_redis_key, job_output_index_redis_key, job_result_value_redis_key},
    },
    quick_fetch::QuickFetch,
};
use redis::{
    Client, TypedCommands,
    streams::{StreamAutoClaimOptions, StreamId, StreamReadOptions},
};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::{
    data::{ACCOUNT_ROW, BUNDLE_DATA, CONFIG, Config, SONG_LIST},
    db::redis::REDIS_CLIENT,
};

pub struct WorkerResult {
    pub friends: Vec<FriendInfo>,
    pub jobs: Vec<Job>,
    pub cursor: usize,
    pub error: Option<Error>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("some data is not ready: {0}")]
    NotReady(String),
    #[error("redis error: {0}")]
    Redis(redis::RedisError),
    #[error("redis extended error: {0}")]
    RedisExtend(cuscuta_common::db::redis::Error),
    #[error("job parse error: {0}")]
    JobParse(db::redis::Error),
    #[error("api error: {0}")]
    Api(api::Error),
    #[error("bad state error: {0}")]
    BadState(String),
}

#[allow(clippy::too_many_lines)]
pub async fn worker_loop(cancellation_token: &CancellationToken) -> WorkerResult {
    async fn internal_loop(
        current_jobs: &mut Vec<Job>,
        cursor: &mut usize,
        friends: &mut Vec<FriendInfo>,
        random: &str,
    ) -> anyhow::Result<(), Error> {
        let redis_client = REDIS_CLIENT
            .get()
            .ok_or(Error::NotReady("redis client".to_string()))?;
        let account_row = ACCOUNT_ROW
            .try_read(std::clone::Clone::clone)
            .map_err(|e| Error::NotReady(format!("account row ({e})")))?;
        let (user_id, token) = account_row
            .check_log_info()
            .map(|(id, token)| (id.to_string(), token))
            .ok_or(Error::NotReady("user is not login".to_string()))?;
        let config = CONFIG
            .try_read(std::clone::Clone::clone)
            .map_err(|e| Error::NotReady(format!("config ({e})")))?;
        let bundle_data = BUNDLE_DATA
            .try_read(std::clone::Clone::clone)
            .map_err(|e| Error::NotReady(format!("bundle data ({e})")))?;
        let song_list = SONG_LIST
            .try_read(std::clone::Clone::clone)
            .map_err(|e| Error::NotReady(format!("song list ({e})")))?;
        let current_sub_queue = &current_jobs.first().map(|it| it.sub_queue.clone());
        let song_list_len = song_list.len();
        let (new_jobs, current_segments) = if let Some(s) = current_sub_queue {
            (
                pull_jobs(
                    current_jobs,
                    s,
                    &config,
                    redis_client,
                    random,
                    song_list_len,
                )?,
                s,
            )
        } else {
            let sub_queues = scan_sub_queue(redis_client).map_err(|e| match e {
                db::redis::Error::Redis(redis_error) => Error::Redis(redis_error),
                db::redis::Error::BadData(e) => Error::BadState(format!("bad data: {e}")),
            })?;
            let Some((jobs, sub_queue)) = discover_sub_queue(
                current_jobs,
                &config,
                redis_client,
                random,
                &sub_queues,
                song_list_len,
            )?
            else {
                sleep(Duration::from_secs(1)).await;
                return Ok(());
            };
            *cursor = sub_queue.segment.start;
            (Some(jobs), &sub_queue.clone())
        };
        if let Some(new_jobs) = new_jobs {
            for it in new_jobs {
                log::info!("pulled job: {it:?}");
                current_jobs.push(it);
            }
        }
        if current_jobs.is_empty() {
            log::debug!("no jobs, skip");
            sleep(Duration::from_secs(2)).await;
            return Ok(());
        }
        try_add_friends(
            &config,
            &bundle_data,
            &user_id,
            &token,
            &account_row,
            current_jobs,
            *cursor,
            friends,
        )
        .await?;
        let rank_list = gather_rank_list(
            &bundle_data,
            &user_id,
            &token,
            &account_row,
            &song_list,
            *cursor,
            &config,
        )
        .await?;
        let linked_result = process_job_with_result(current_jobs, &rank_list);
        write_result_to_redis(redis_client, &linked_result)?;
        refresh_redis_ttl(current_jobs, redis_client, &config)?;
        clean_jobs(
            current_jobs,
            friends,
            redis_client,
            &bundle_data,
            &user_id,
            &token,
            &account_row,
            &config,
        )
        .await?;
        *cursor += 1;
        if *cursor >= current_segments.segment.end {
            *cursor = current_segments.segment.start;
        }
        Ok(())
    }
    //TODO optimize error handling
    let mut friends = Vec::<FriendInfo>::new();
    let mut current_jobs = Vec::<Job>::new();
    let mut cursor = 0;
    let random = rand::random::<u64>().to_string();
    while !cancellation_token.is_cancelled() {
        if let Err(e) = internal_loop(&mut current_jobs, &mut cursor, &mut friends, &random).await {
            if let Error::Api(api_error) = &e {
                match api_error {
                    api::Error::Network(_) => {}
                    _ => {
                        return WorkerResult {
                            friends,
                            jobs: current_jobs,
                            cursor,
                            error: Some(e),
                        };
                    }
                }
            }
            log::warn!("worker loop failed: {e}");
            sleep(Duration::from_secs(1)).await;
        }
    }
    WorkerResult {
        friends,
        jobs: current_jobs,
        cursor,
        error: None,
    }
}

fn refresh_redis_ttl(jobs: &[Job], redis_client: &Client, config: &Config) -> Result<(), Error> {
    let mut pipe = redis::pipe();
    for job in jobs {
        pipe.expire(
            job_output_index_redis_key(&job.get_stream_key_postfix()),
            config.redis_stream_refresh_ttl,
        )
        .expire(
            job_result_value_redis_key(&job.get_stream_key_postfix()),
            config.redis_stream_refresh_ttl,
        )
        .expire(
            job_pending_index_redis_key(&job.get_stream_key_postfix()),
            config.redis_stream_refresh_ttl,
        );
    }
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    pipe.exec(&mut connection).map_err(Error::Redis)
}

#[allow(clippy::too_many_arguments)]
async fn clean_jobs(
    jobs: &mut Vec<Job>,
    friends: &mut Vec<FriendInfo>,
    redis_client: &Client,
    bundle_data: &BundleData,
    user_id: &str,
    token: &str,
    account_row: &AccountRow,
    config: &Config,
) -> Result<(), Error> {
    let pending_friends: Vec<_> = jobs
        .iter()
        .filter_map(|it| match it.state {
            JobState::Pulled { .. } | JobState::Pending { .. } => {
                Some(it.essential.friend_code.clone())
            }
            _ => None,
        })
        .collect();
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    for finished_job in jobs.iter_mut() {
        let output_list_key = job_output_index_redis_key(&finished_job.get_stream_key_postfix());
        let JobState::Finished {
            friend_user_id,
            start_timestamp,
        } = &finished_job.state
        else {
            continue;
        };
        connection
            .xack(
                finished_job.sub_queue.name.clone(),
                "default_group",
                std::slice::from_ref(&finished_job.job_id),
            )
            .map_err(Error::Redis)?;
        let json = serde_json::to_string(&JobTag {
            queue: finished_job.sub_queue.clone(),
            job_essential: finished_job.essential.clone(),
            job_last_id: finished_job.job_id.clone(),
        })
        .map_err(|e| Error::BadState(format!("failed to serialize data to json: {e}")))?;
        connection
            .lpush(output_list_key.clone(), &json)
            .map_err(Error::Redis)?;
        let _ = connection.expire(output_list_key, config.redis_stream_refresh_ttl);
        if !pending_friends.contains(&finished_job.essential.friend_code) {
            xxxxxx_safe_call(
                config.worker_max_retry_count,
                config.worker_exponential_backoff_base_millis,
                config.worker_exponential_backoff_multiplier,
                config.worker_exponential_backoff_max_delay_millis,
                || {
                    api_delete_friend(
                        bundle_data,
                        &account_row.account_email,
                        user_id,
                        token,
                        friend_user_id,
                    )
                },
            )
            .await
            .map_err(Error::Api)?;
            friends.retain(|it| &it.user_id.to_string() != friend_user_id);
        }
        let _ = record_eta(
            redis_client,
            (Utc::now().timestamp_millis() - *start_timestamp)
                / i64::from(finished_job.essential.cursor_length),
        );
        log::info!("job: {finished_job:?} finished");
        finished_job.state = JobState::Cleaned;
    }
    jobs.retain(|it| it.state != JobState::Cleaned);
    Ok(())
}

fn process_job_with_result(jobs: &mut [Job], scores: &[SongScore]) -> Vec<(String, SongScore)> {
    let mut job_links = Vec::new();
    for job in jobs.iter_mut() {
        let redis_key = job_result_value_redis_key(&job.get_stream_key_postfix());
        let JobState::Pending {
            friend_user_id,
            current_length,
            start_timestamp,
        } = &mut job.state
        else {
            continue;
        };
        let linked_score = scores
            .iter()
            .filter(|it| friend_user_id == &it.user_id.to_string());
        *current_length += 1;
        for linked_score in linked_score {
            job_links.push((redis_key.clone(), linked_score.clone()));
        }
        if *current_length >= job.essential.cursor_length.cast_unsigned() as usize {
            job.state = JobState::Finished {
                friend_user_id: friend_user_id.clone(),
                start_timestamp: *start_timestamp,
            };
        }
    }
    log::debug!(
        "linked {} results for {:?}",
        job_links.len(),
        scores.first().map(|it| it.song_id.clone())
    );
    job_links
}

fn write_result_to_redis(
    redis_client: &Client,
    score_pairs: &[(String, SongScore)],
) -> Result<(), Error> {
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    for (key, score) in score_pairs {
        let json = serde_json::to_string(score)
            .map_err(|e| Error::BadState(format!("failed to serialize data to json: {e}")))?;
        connection.lpush(key, &json).map_err(Error::Redis)?;
    }
    Ok(())
}

async fn gather_rank_list<'a>(
    bundle_data: &'a BundleData,
    user_id: &'a str,
    token: &'a str,
    account_row: &'a AccountRow,
    song_list: &'a [Song],
    cursor: usize,
    config: &Config,
) -> Result<Vec<SongScore>, Error> {
    let Some(song) = song_list.get(cursor) else {
        return Ok(Vec::new());
    };
    let mut result = Vec::new();
    for difficulty in &song.difficulties {
        let rating_class = difficulty.rating_class.to_string();
        let rank_list = xxxxxx_safe_call(
            config.worker_max_retry_count,
            config.worker_exponential_backoff_base_millis,
            config.worker_exponential_backoff_multiplier,
            config.worker_exponential_backoff_max_delay_millis,
            || {
                api::xxxxxx::api_get_rank_list(
                    bundle_data,
                    &account_row.account_email,
                    user_id,
                    token,
                    &song.id,
                    &rating_class,
                    "0",
                    "11",
                )
            },
        )
        .await
        .map_err(Error::Api)?;
        for it in rank_list {
            result.push(it);
        }
    }
    Ok(result)
}

#[allow(clippy::cast_possible_truncation, clippy::too_many_arguments)]
async fn try_add_friends(
    config: &Config,
    bundle_data: &BundleData,
    user_id: &str,
    token: &str,
    account_row: &AccountRow,
    jobs: &mut [Job],
    cursor: usize,
    friends: &mut Vec<FriendInfo>,
) -> Result<(), Error> {
    let ids: Vec<_> = jobs
        .iter()
        .filter_map(|it| {
            let (JobState::Pending { friend_user_id, .. }
            | JobState::Finished { friend_user_id, .. }) = &it.state
            else {
                return None;
            };
            Some((it.essential.friend_code.clone(), friend_user_id.clone()))
        })
        .collect();
    for job in jobs.iter_mut() {
        let JobState::Pulled { start_timestamp } = job.state else {
            continue;
        };
        let option_existing_ids = ids
            .iter()
            .find(|(code, _)| code == &job.essential.friend_code);
        if let Some((_, existing_friend_id)) = option_existing_ids {
            job.state = JobState::Pending {
                friend_user_id: existing_friend_id.clone(),
                start_timestamp,
                current_length: 0,
            };
            job.essential.cursor_start = cursor.cast_signed() as i32;
            continue;
        }
        let result = xxxxxx_safe_call(
            config.worker_max_retry_count,
            config.worker_exponential_backoff_base_millis,
            config.worker_exponential_backoff_multiplier,
            config.worker_exponential_backoff_max_delay_millis,
            || {
                api::xxxxxx::api_add_friend(
                    bundle_data,
                    &account_row.account_email,
                    user_id,
                    token,
                    &job.essential.friend_code,
                )
            },
        )
        .await;
        let friends_new = match result {
            Err(e) => {
                if let api::Error::BadStatus(code) = &e
                    && *code == 400
                {
                    log::warn!("friend is already exist but cache is out-dated!");
                    continue;
                }
                log::warn!("failed to add friend: {e}");
                continue;
            }
            Ok(it) => it.friends,
        };
        let friend_delta = calc_friend_delta(friends, &friends_new)
            .map_err(|e| Error::BadState(format!("failed to resolve friend delta: {e}")))?;
        let friend_add = match friend_delta {
            FriendDelta::Add(it) => it,
            FriendDelta::Remove(e) => {
                return Err(Error::BadState(format!(
                    "bad friend delta (the friend dropped... what?) {e:?}"
                )));
            }
            FriendDelta::Same => {
                return Err(Error::BadState(
                    "bad friend delta (nothing changed)".to_string(),
                ));
            }
        };
        *friends = friends_new;
        job.essential.cursor_start = cursor.cast_signed() as i32;
        job.state = JobState::Pending {
            friend_user_id: friend_add.user_id.to_string(),
            start_timestamp,
            current_length: 0,
        };
    }
    Ok(())
}

fn discover_sub_queue(
    jobs: &[Job],
    config: &Config,
    redis_client: &Client,
    pod_uid: &str,
    sub_queues: &[SubQueue],
    total_length: usize,
) -> Result<Option<(Vec<Job>, SubQueue)>, Error> {
    for queue in sub_queues {
        let Some(jobs) = pull_jobs(jobs, queue, config, redis_client, pod_uid, total_length)?
        else {
            continue;
        };
        if jobs.is_empty() {
            continue;
        }
        return Ok(Some((jobs, queue.clone())));
    }
    Ok(None)
}

fn valid_jobs(jobs: &[Job]) -> usize {
    jobs.iter()
        .filter(|it| it.state != JobState::Cleaned)
        .count()
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::too_many_lines
)]
fn pull_jobs(
    jobs: &[Job],
    sub_queue: &SubQueue,
    config: &Config,
    redis_client: &Client,
    pod_uid: &str,
    total_length: usize,
) -> Result<Option<Vec<Job>>, Error> {
    // TODO: 添加无GROUP找不到的错误处理（跳过）
    fn claim_jobs(
        connection: &mut redis::Connection,
        key: &str,
        consumer: &str,
        min_idle_time_secs: f64,
        target_counts: usize,
    ) -> Result<Vec<StreamId>, Error> {
        if target_counts == 0 {
            return Ok(Vec::new());
        }
        Ok(connection
            .xautoclaim_options(
                key,
                "default_group",
                consumer,
                (min_idle_time_secs * 1000.0) as i64,
                "0-0",
                StreamAutoClaimOptions::default().count(target_counts),
            )
            .map_err(Error::Redis)?
            .claimed)
    }
    fn fetch_jobs(
        connection: &mut redis::Connection,
        key: &str,
        consumer: &str,
        target_counts: usize,
    ) -> Result<Vec<StreamId>, Error> {
        if target_counts == 0 {
            return Ok(Vec::new());
        }
        Ok(connection
            .xread_options(
                &[&key],
                &[">"],
                &StreamReadOptions::default()
                    .group("default_group", consumer)
                    .count(target_counts),
            )
            .map_err(Error::Redis)?
            .map_or(Vec::new(), |it| {
                it.keys.into_iter().next().map_or(Vec::new(), |it| it.ids)
            }))
    }
    let valid_jobs = valid_jobs(jobs);
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    let max_jobs = config.worker_max_jobs.try_into().expect("wait... what?");
    if valid_jobs >= 1 {
        let _ = connection.expire(sub_queue.name.clone(), config.redis_stream_refresh_ttl);
    }
    if valid_jobs >= max_jobs {
        return Ok(Option::None);
    }
    let divisions = jobs
        .first()
        .map_or(1, |it| {
            (total_length as f64 / it.sub_queue.segment.len() as f64) as i32
        })
        .max(1);
    let min_idle_time =
        (config.worker_job_max_work_time_secs as f64 / f64::from(divisions)).round();
    let claimed_jobs = match claim_jobs(
        &mut connection,
        &sub_queue.name,
        pod_uid,
        min_idle_time,
        max_jobs.saturating_sub(valid_jobs),
    ) {
        Ok(o) => o,
        Err(e) => {
            log::warn!("worker_loop_pull_jobs: failed to claim jobs: {e}");
            Vec::new()
        }
    };
    let valid_jobs = valid_jobs + claimed_jobs.len();
    let fetched_jobs = match fetch_jobs(
        &mut connection,
        &sub_queue.name,
        pod_uid,
        max_jobs.saturating_sub(valid_jobs),
    ) {
        Ok(o) => o,
        Err(e) => {
            log::warn!("worker_loop_pull_jobs: failed to fetch jobs: {e}");
            Vec::new()
        }
    };
    let jobs: Vec<_> = fetched_jobs
        .iter()
        .chain(claimed_jobs.iter())
        .map(|it| {
            (sub_queue.clone(), it.clone())
                .try_into()
                .map_err(Error::JobParse)
        })
        .collect::<Result<Vec<Job>, Error>>()?;
    let mut no_duplicated_jobs = HashMap::new();
    for job in &jobs {
        no_duplicated_jobs.entry(&job.essential).or_insert(job);
    }
    Ok(Some(
        no_duplicated_jobs
            .into_iter()
            .collect::<Vec<(_, _)>>()
            .into_iter()
            .map(|it| it.1.clone())
            .collect(),
    ))
}

pub fn resume_state(worker_result: WorkerResult) {
    fn resume_jobs(worker_result: WorkerResult) -> Result<(), Error> {
        let redis_client = REDIS_CLIENT
            .get()
            .ok_or(Error::NotReady("redis client".to_string()))?;
        let redis_stream_refresh_ttl = CONFIG
            .try_read(|it| it.redis_stream_refresh_ttl)
            .unwrap_or(300);
        let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
        for job in worker_result.jobs {
            log::info!("resuming_jobs: reenqueueing job: {job:?}");
            connection
                .xack(&job.sub_queue.name, "default_group", &[&job.job_id])
                .map_err(Error::Redis)?;
            let job_id = write_job(
                redis_client,
                &job.essential,
                &job.sub_queue.name,
                false,
                redis_stream_refresh_ttl,
            )
            .map_err(Error::Redis)?;
            write_job_index(
                redis_client,
                &JobTag {
                    job_last_id: job_id,
                    queue: job.sub_queue,
                    job_essential: job.essential,
                },
            )
            .map_err(Error::RedisExtend)?;
        }
        Ok(())
    }
    if let Err(e) = resume_jobs(worker_result) {
        log::error!("resuming: failed to resume jobs: {e}");
    }
}
