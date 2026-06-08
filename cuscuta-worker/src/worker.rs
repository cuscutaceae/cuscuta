use std::time::Duration;

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
        account::AccountRow,
        job::{self, Job, JobTag, SubQueue, write_job},
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
    #[error("job parse error: {0}")]
    JobParse(job::Error),
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
        let (new_jobs, current_segments) = if let Some(s) = current_sub_queue {
            (
                pull_jobs(current_jobs, s, &config, redis_client, random)?,
                s,
            )
        } else {
            let sub_queues = job::scan_sub_queue(redis_client).map_err(|e| match e {
                job::Error::Redis(redis_error) => Error::Redis(redis_error),
                job::Error::BadData(e) => Error::BadState(format!("bad data: {e}")),
            })?;
            let Some((jobs, sub_queue)) =
                discover_sub_queue(current_jobs, &config, redis_client, random, &sub_queues)?
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
        write_result_to_redis(redis_client, &config, &linked_result)?;
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
            sleep(Duration::from_secs(10)).await;
        }
    }
    WorkerResult {
        friends,
        jobs: current_jobs,
        cursor,
        error: None,
    }
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
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    for finished_job in jobs.iter_mut().filter(|it| it.finished && !it.cleaned) {
        connection
            .xack(
                finished_job.sub_queue.name.clone(),
                "default_group",
                std::slice::from_ref(&finished_job.job_id),
            )
            .map_err(Error::Redis)?;
        let output_list_key = format!(
            "cuscuta:results:index:{}-{}",
            finished_job.friend_code.clone(),
            finished_job.timestamp.clone()
        );
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
        if let Some(friend_id) = &finished_job.friend_user_id {
            api_delete_friend(
                bundle_data,
                account_row.account_email.clone(),
                user_id.to_string(),
                token.to_string(),
                friend_id.clone(),
            )
            .await
            .map_err(Error::Api)?;
            friends.retain(|it| &it.user_id.to_string() != friend_id);
        }
        log::info!("job: {finished_job:?} finished");
        finished_job.cleaned = true;
    }
    jobs.retain(|it| !it.cleaned);
    Ok(())
}

fn process_job_with_result(jobs: &mut [Job], scores: &[SongScore]) -> Vec<(String, SongScore)> {
    let mut job_links = Vec::new();
    for job in jobs {
        let linked_score = scores
            .iter()
            .filter(|it| job.friend_user_id.clone().unwrap_or_default() == it.user_id.to_string());
        job.current_length += 1;
        if job.current_length >= job.cursor_length.cast_unsigned() as usize {
            job.finished = true;
        }
        for linked_score in linked_score {
            job_links.push((
                format!(
                    "cuscuta:results:value:{}-{}",
                    job.friend_code.clone(),
                    job.timestamp.clone()
                ),
                linked_score.clone(),
            ));
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
    config: &Config,
    score_pairs: &[(String, SongScore)],
) -> Result<(), Error> {
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    for (key, score) in score_pairs {
        let json = serde_json::to_string(score)
            .map_err(|e| Error::BadState(format!("failed to serialize data to json: {e}")))?;
        connection.lpush(key, &json).map_err(Error::Redis)?;
        let _ = connection.expire(key, config.redis_stream_refresh_ttl);
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
        let rank_list = xxxxxx_safe_call(
            config.worker_max_retry_count,
            config.worker_exponential_backoff_base_millis,
            config.worker_exponential_backoff_multiplier,
            config.worker_exponential_backoff_max_delay_millis,
            || {
                api::xxxxxx::api_get_rank_list(
                    bundle_data,
                    account_row.account_email.clone(),
                    user_id.to_string(),
                    token.to_string(),
                    song.id.clone(),
                    difficulty.rating_class.to_string(),
                    "0".into(),
                    "11".into(),
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

#[allow(clippy::cast_possible_truncation)]
async fn try_add_friends(
    bundle_data: &BundleData,
    user_id: &str,
    token: &str,
    account_row: &AccountRow,
    jobs: &mut [Job],
    cursor: usize,
    friends: &mut Vec<FriendInfo>,
) -> Result<(), Error> {
    for job in jobs
        .iter_mut()
        .filter(|it| !it.friend_added && !it.finished)
    {
        let result = api::xxxxxx::api_add_friend(
            bundle_data,
            account_row.account_email.clone(),
            user_id.to_string(),
            token.to_string(),
            job.friend_code.clone(),
        )
        .await;
        let friends_new = match result {
            Err(e) => {
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
        job.friend_user_id = Some(friend_add.user_id.to_string());
        job.cursor_start = cursor.cast_signed() as i32;
        job.friend_added = true;
    }
    Ok(())
}

fn discover_sub_queue(
    jobs: &[Job],
    config: &Config,
    redis_client: &Client,
    pod_uid: &str,
    sub_queues: &[SubQueue],
) -> Result<Option<(Vec<Job>, SubQueue)>, Error> {
    for queue in sub_queues {
        let Some(jobs) = pull_jobs(jobs, queue, config, redis_client, pod_uid)? else {
            continue;
        };
        return Ok(Some((jobs, queue.clone())));
    }
    Ok(None)
}

fn valid_jobs(jobs: &[Job]) -> usize {
    jobs.iter().filter(|it| !it.cleaned).count()
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn pull_jobs(
    jobs: &[Job],
    sub_queue: &SubQueue,
    config: &Config,
    redis_client: &Client,
    pod_uid: &str,
) -> Result<Option<Vec<Job>>, Error> {
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
    let divisions = jobs.first().map_or(1, |it| it.cursor_length).max(1);
    let min_idle_time =
        (config.worker_job_max_work_time_secs as f64 / f64::from(divisions)).round();
    let claimed_jobs = claim_jobs(
        &mut connection,
        &sub_queue.name,
        pod_uid,
        min_idle_time,
        max_jobs.saturating_sub(valid_jobs),
    )?;
    let valid_jobs = valid_jobs + claimed_jobs.len();
    let fetched_jobs = fetch_jobs(
        &mut connection,
        &sub_queue.name,
        pod_uid,
        max_jobs.saturating_sub(valid_jobs),
    )?;
    let jobs: Vec<_> = fetched_jobs.iter().chain(claimed_jobs.iter()).collect();
    let mut add_jobs = Vec::new();
    for job_id in jobs {
        let job: Job = (sub_queue.clone(), job_id.clone())
            .try_into()
            .map_err(Error::JobParse)?;
        add_jobs.push(job);
    }
    Ok(Some(add_jobs))
}

pub fn resume_state(worker_result: &WorkerResult) {
    fn resume_jobs(worker_result: &WorkerResult) -> Result<(), Error> {
        let redis_client = REDIS_CLIENT
            .get()
            .ok_or(Error::NotReady("redis client".to_string()))?;
        let redis_stream_refresh_ttl = CONFIG
            .try_read(|it| it.redis_stream_refresh_ttl)
            .unwrap_or(300);
        let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
        for job in &worker_result.jobs {
            log::info!("resuming_jobs: reenqueueing job: {job:?}");
            connection
                .xack(&job.sub_queue.name, "default_group", &[&job.job_id])
                .map_err(Error::Redis)?;
            write_job(
                redis_client,
                &job.essential,
                job.sub_queue.name.clone(),
                false,
                redis_stream_refresh_ttl,
            )
            .map_err(Error::Redis)?;
        }
        Ok(())
    }
    if let Err(e) = resume_jobs(worker_result) {
        log::error!("resuming: failed to resume jobs: {e}");
    }
}
