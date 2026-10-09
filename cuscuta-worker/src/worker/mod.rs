mod clean;
mod friend_modify;
mod pending_friend;
mod pending_gather;
mod pull;

use std::time::Duration;

use cuscuta_common::{
    api::{
        self,
        xxxxxx::{FriendInfo, XxxxxxUrl},
    },
    data::{AppVersionData, SongsWithHash},
    db::{
        self,
        account::AccountRow,
        job::{
            Job, JobFailure, JobFailureResuming, JobFailureType,
            enqueue::write_job,
            track::{
                JobTrackQueueStatus, JobTrackTag, batch_write_job_tracking_tag, fetch_job_track_tag,
            },
        },
        log::status::update_worker_status,
        redis::{
            job_result_friend_info_redis_key, job_result_tracking_redis_key,
            job_result_value_redis_key,
        },
    },
    quick_fetch::QuickFetch,
};
use redis::{Client, TypedCommands};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::{
    config::{Environment, fetch_env},
    data::{ACCOUNT_ROW, APP_VERSION_DATA, SONG_LIST, WORKER_ID, XXXXXX_URL},
    db::redis::REDIS_CLIENT,
    worker::{
        clean::clean_jobs,
        pending_friend::try_add_friends,
        pending_gather::{gather_rank_list, process_job_with_result, write_result_to_redis},
        pull::scan_sub_queue_and_pull_job,
    },
};

#[derive(Debug)]
pub struct WorkerResult {
    pub friends: Vec<FriendInfo>,
    pub jobs: Vec<Job>,
    pub cursor: usize,
    pub error: Option<Error>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("some data is not ready: {message}")]
    NotReady { message: String },

    #[error("redis error: {0}")]
    Redis(redis::RedisError),

    #[error("redis extended error: {0}")]
    RedisExtend(cuscuta_common::db::redis::Error),

    #[error("job parse error: {0}")]
    JobParse(db::redis::Error),

    #[error("json parsing error: {0}")]
    JsonParse(serde_json::Error),

    #[error("api error: {0}")]
    Api(api::Error),

    #[error("bad state error: {message}")]
    BadState { message: String },
}

pub async fn worker_loop(cancellation_token: &CancellationToken) -> WorkerResult {
    //TODO optimize error handling
    let mut friends = Vec::<FriendInfo>::new();
    let mut current_jobs = Vec::<Job>::new();
    let mut cursor = 0;
    let worker_id = format!(
        "{}-{}",
        gethostname::gethostname().to_str().unwrap_or("unknown"),
        rand::random::<u64>()
    );
    WORKER_ID.get_or_init(|| worker_id.clone());
    while !cancellation_token.is_cancelled() {
        if let Err(e) = internal_loop(&mut current_jobs, &mut cursor, &mut friends).await {
            tracing::error!("worker loop failed: {e}");
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
            sleep(Duration::from_secs(1)).await;
        }
    }
    let worker_result = WorkerResult {
        friends,
        jobs: current_jobs,
        cursor,
        error: None,
    };
    tracing::error!("worker down: {worker_result:?}");
    worker_result
}

struct Args<'a> {
    worker_id: &'a String,
    redis_client: &'a Client,
    account_row: AccountRow,
    user_id: String,
    token: String,
    bundle_data: AppVersionData,
    xxxxxx_url: XxxxxxUrl,
    songs_with_hash_pool: Vec<SongsWithHash>,
}

async fn get_args<'a>() -> Result<Args<'a>, Error> {
    let worker_id = WORKER_ID.get().ok_or(Error::NotReady {
        message: "worker_id... what?".to_string(),
    })?;
    let redis_client = REDIS_CLIENT.get().ok_or(Error::NotReady {
        message: "redis client".to_string(),
    })?;
    let account_row = ACCOUNT_ROW
        .read_spinning(std::clone::Clone::clone)
        .await
        .map_err(|e| Error::NotReady {
            message: format!("account row ({e})"),
        })?;
    let (user_id, token) = account_row
        .check_log_info()
        .map(|(id, token)| (id.to_string(), token))
        .ok_or(Error::NotReady {
            message: "user is not login".to_string(),
        })?;
    let bundle_data = APP_VERSION_DATA
        .read_spinning(std::clone::Clone::clone)
        .await
        .map_err(|e| Error::NotReady {
            message: format!("bundle data ({e})"),
        })?;
    let songs_with_hash_pool = SONG_LIST
        .read_spinning(std::clone::Clone::clone)
        .await
        .map_err(|e| Error::NotReady {
            message: format!("song list ({e})"),
        })?;
    let xxxxxx_url = XXXXXX_URL
        .read_spinning(std::clone::Clone::clone)
        .await
        .map_err(|e| Error::NotReady {
            message: format!("xxxxxx_url ({e})"),
        })?;
    Ok(Args {
        worker_id,
        redis_client,
        account_row,
        user_id,
        token,
        bundle_data,
        xxxxxx_url,
        songs_with_hash_pool,
    })
}

async fn internal_loop(
    current_jobs: &mut Vec<Job>,
    cursor: &mut usize,
    friends: &mut Vec<FriendInfo>,
) -> anyhow::Result<(), Error> {
    let Args {
        worker_id,
        redis_client,
        account_row,
        user_id,
        token,
        bundle_data,
        xxxxxx_url,
        songs_with_hash_pool,
    } = get_args().await?;
    let Some((current_segments, songs_with_hash)) = scan_sub_queue_and_pull_job(
        redis_client,
        current_jobs,
        cursor,
        &songs_with_hash_pool,
        worker_id,
    )
    .await?
    else {
        if let Err(e) = update_worker_status(redis_client, worker_id, *cursor, None, current_jobs) {
            tracing::warn!("worker_loop: failed to update worker status #1: {e}");
        }
        sleep(Duration::from_secs(1)).await;
        return Ok(());
    };
    if let Err(e) = update_worker_status(
        redis_client,
        worker_id,
        *cursor,
        Some(&current_segments),
        current_jobs,
    ) {
        tracing::warn!("worker_loop: failed to update worker status #2: {e}");
    }
    if current_jobs.is_empty() {
        tracing::debug!("worker_loop: no jobs, skip");
        sleep(Duration::from_secs(2)).await;
        return Ok(());
    }
    try_add_friends(
        &xxxxxx_url,
        &bundle_data,
        redis_client,
        &user_id,
        &token,
        &account_row,
        current_jobs,
        *cursor,
        friends,
    )
    .await?;
    let rank_list = if current_segments.segment.is_empty() {
        vec![]
    } else {
        gather_rank_list(
            &xxxxxx_url,
            &bundle_data,
            &user_id,
            &token,
            &account_row,
            songs_with_hash,
            *cursor,
        )
        .await?
    };
    let linked_result = process_job_with_result(current_jobs, &rank_list);
    write_result_to_redis(redis_client, &linked_result)?;
    refresh_redis_ttl(current_jobs, redis_client)?;
    clean_jobs(
        current_jobs,
        friends,
        redis_client,
        &bundle_data,
        &user_id,
        &token,
        &account_row,
        &xxxxxx_url,
    )
    .await?;
    *cursor += 1;
    if *cursor >= current_segments.segment.end {
        *cursor = current_segments.segment.start;
    }
    Ok(())
}

fn refresh_redis_ttl(jobs: &[Job], redis_client: &Client) -> Result<(), Error> {
    let config = fetch_env();
    let mut pipe = redis::pipe();
    for job in jobs {
        pipe.expire(
            job_result_tracking_redis_key(&job.get_stream_key_postfix()),
            config.redis_stream_refresh_ttl,
        )
        .expire(
            job_result_value_redis_key(&job.get_stream_key_postfix()),
            config.redis_stream_refresh_ttl,
        )
        .expire(
            job_result_friend_info_redis_key(&job.get_stream_key_postfix()),
            config.redis_stream_refresh_ttl,
        );
    }
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    pipe.exec(&mut connection).map_err(Error::Redis)
}

fn update_job_track_info<F, D>(
    redis_client: &Client,
    job: &Job,
    mut transform_fun: F,
    default_fun: D,
) -> Result<(), Error>
where
    F: FnMut(&mut JobTrackTag),
    D: FnOnce() -> JobTrackTag,
{
    let info = fetch_job_track_tag(redis_client, job).map_or_else(
        |_| default_fun(),
        |mut it| {
            transform_fun(&mut it);
            it
        },
    );
    batch_write_job_tracking_tag(redis_client, &[info]).map_err(Error::RedisExtend)
}

pub async fn resume_state(worker_result: WorkerResult) {
    // TODO: complete error handling here
    fn resume_jobs(worker_result: WorkerResult) -> Result<(), Error> {
        let redis_client = REDIS_CLIENT.get().ok_or(Error::NotReady {
            message: "redis client".to_string(),
        })?;
        let redis_stream_refresh_ttl = fetch_env().redis_stream_refresh_ttl;
        let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
        for job in worker_result.jobs {
            tracing::info!("resuming_jobs: reenqueueing job: {job:?}");
            connection
                .xack(&job.sub_queue.name, "default_group", &[&job.job_id])
                .map_err(Error::Redis)?;
            let job_id = write_job(
                redis_client,
                &job.essential,
                &job.sub_queue.get_postfix(),
                false,
                redis_stream_refresh_ttl,
            )
            .map_err(Error::Redis)?;
            update_job_track_info(
                redis_client,
                &job,
                |o| {
                    o.status = JobTrackQueueStatus::Success;
                    o.job_ids.push(job_id.clone());
                },
                || JobTrackTag {
                    status: JobTrackQueueStatus::Queueing,
                    job_ids: vec![job.job_id.clone(), job_id.clone()],
                    queue: job.sub_queue.clone(),
                    job_essential: job.essential.clone(),
                    failures: vec![JobFailure::new(
                        JobFailureType::Reenqueued,
                        JobFailureResuming::NoOp,
                    )],
                },
            )?;
        }
        Ok(())
    }
    if let Err(e) = resume_jobs(worker_result) {
        tracing::error!("resuming: failed to resume jobs: {e}");
    }
}
