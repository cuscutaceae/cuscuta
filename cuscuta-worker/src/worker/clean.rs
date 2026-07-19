use chrono::Utc;
use cuscuta_common::{
    api::xxxxxx::{FriendInfo, api_delete_friend, auto::xxxxxx_safe_call},
    data::BundleData,
    db::{
        account::AccountRow,
        job::{
            Job, JobFailure, JobFailureResuming, JobFailureType, JobState,
            eta::record_eta,
            track::{JobTrackQueueStatus, JobTrackTag},
        },
        log::WorkerEventType,
    },
};
use redis::{Client, TypedCommands};

use crate::{
    data::Config,
    worker::{Error, update_job_track_info},
    worker_write_event,
};

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub async fn clean_jobs(
    jobs: &mut Vec<Job>,
    friends: &mut Vec<FriendInfo>,
    redis_client: &Client,
    bundle_data: &BundleData,
    user_id: &str,
    token: &str,
    account_row: &AccountRow,
    config: &Config,
) -> Result<(), Error> {
    let pending_friends_code = get_pending_friends_code(jobs);
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    for finished_job in jobs.iter_mut() {
        let (friend_info, start_timestamp, failure_info) = match &finished_job.state {
            JobState::Finished {
                friend_info,
                start_timestamp,
            } => (Some(friend_info), start_timestamp, None),
            JobState::Failed {
                friend_info,
                start_timestamp,
                failure_info,
            } => (friend_info.as_ref(), start_timestamp, Some(failure_info)),
            _ => continue,
        };
        connection
            .xack(
                finished_job.sub_queue.name.clone(),
                "default_group",
                std::slice::from_ref(&finished_job.job_id),
            )
            .map_err(Error::Redis)?;
        if let Err(e) = update_job_track_info(
            redis_client,
            finished_job,
            |tag| {
                if let Some(failure_info) = failure_info {
                    tag.status = JobTrackQueueStatus::Failed;
                    tag.failures.push(failure_info.clone());
                } else {
                    tag.status = JobTrackQueueStatus::Success;
                }
            },
            || JobTrackTag {
                status: failure_info.map_or(JobTrackQueueStatus::Success, |_| {
                    JobTrackQueueStatus::Failed
                }),
                job_ids: vec![finished_job.job_id.clone()],
                queue: finished_job.sub_queue.clone(),
                job_essential: finished_job.essential.clone(),
                failures: [
                    Some(JobFailure::new(
                        JobFailureType::TargetKeyNotFound("finish_job".to_owned()),
                        JobFailureResuming::NoOp,
                    )),
                    failure_info.cloned(),
                ]
                .into_iter()
                .flatten()
                .collect(),
            },
        ) {
            tracing::warn!("job_clean: failed to write track tag: {e}");
            continue;
        }
        if let Some(friend_info) = friend_info
            && !pending_friends_code.contains(&finished_job.essential.friend_code)
        {
            let friend_user_id = friend_info.user_id.to_string();
            if let Err(e) = xxxxxx_safe_call(
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
                        &friend_user_id,
                    )
                },
            )
            .await
            {
                worker_write_event!(
                    WorkerEventType::Warn,
                    format!("failed to delete friend: {e}")
                );
            }
            friends.retain(|it| it.user_id != friend_info.user_id);
        }
        let cursor_length = i64::from(finished_job.essential.cursor_length);
        if cursor_length != 0 {
            let _ = record_eta(
                redis_client,
                (Utc::now().timestamp_millis() - *start_timestamp) / cursor_length,
            );
        }
        if let Some(failure_info) = failure_info {
            worker_write_event!(
                WorkerEventType::Warn,
                format!("job finished with error: {failure_info:?}")
            );
            tracing::warn!(
                "job: {finished_job:?} finished with error: {finished_job:?} : {failure_info:?}"
            );
        } else {
            worker_write_event!(
                WorkerEventType::Trace,
                format!("job finished: {finished_job:?}")
            );
            tracing::info!("job: {finished_job:?} finished");
        }
        finished_job.state = JobState::Cleaned;
    }
    jobs.retain(|it| !matches!(it.state, JobState::Cleaned));
    Ok(())
}

fn get_pending_friends_code(jobs: &[Job]) -> Vec<String> {
    jobs.iter()
        .filter_map(|it| match it.state {
            JobState::Pulled { .. } | JobState::Pending { .. } => {
                Some(it.essential.friend_code.clone())
            }
            _ => None,
        })
        .collect()
}
