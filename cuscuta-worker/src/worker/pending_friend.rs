//! 这个文件写的好脏……

use std::collections::HashMap;

use cuscuta_common::{
    api::xxxxxx::{FriendDelta, FriendInfo, calc_friend_delta},
    data::BundleData,
    db::{
        account::AccountRow,
        job::{Job, JobState},
        log::WorkerEventType,
        redis::job_result_friend_info_redis_key,
    },
};
use redis::{Client, Connection, TypedCommands};

use crate::{
    data::Config,
    worker::{
        Error,
        friend_modify::{ExpectedModify, try_modify_remote_friend},
    },
    worker_write_event,
};

#[allow(clippy::cast_possible_truncation, clippy::too_many_arguments)]
pub async fn try_add_friends(
    config: &Config,
    bundle_data: &BundleData,
    redis_client: &Client,
    user_id: &str,
    token: &str,
    account_row: &AccountRow,
    jobs: &mut [Job],
    cursor: usize,
    friends: &mut Vec<FriendInfo>,
) -> Result<(), Error> {
    let mut connection = redis_client.get_connection().map_err(Error::Redis)?;
    let mut ids: HashMap<_, _> = jobs
        .iter()
        .filter_map(|it| {
            let (JobState::Pending { friend_info, .. } | JobState::Finished { friend_info, .. }) =
                &it.state
            else {
                return None;
            };
            Some((it.essential.friend_code.clone(), friend_info.clone()))
        })
        .collect();
    for job in jobs.iter_mut() {
        let JobState::Pulled { start_timestamp } = job.state else {
            continue;
        };
        if let Some(existing_friend_info) = ids.get(&job.essential.friend_code) {
            let friend_info = existing_friend_info.clone();
            push_friend_info(&mut connection, &friend_info, job)?;
            job.state = JobState::Pending {
                friend_info: friend_info.clone(),
                start_timestamp,
                current_length: 0,
            };
            job.essential.cursor_start = cursor.cast_signed() as i32;
            continue;
        }

        let friends_new = match try_modify_remote_friend(
            config,
            bundle_data,
            user_id,
            token,
            account_row,
            ExpectedModify::Add {
                friend_code: job.essential.friend_code.clone(),
            },
            friends,
        )
        .await
        {
            Ok(o) => o,
            Err(failure_info) => {
                job.state = JobState::Failed {
                    start_timestamp,
                    failure_info,
                    friend_info: None,
                };
                continue;
            }
        };
        let friend_delta =
            calc_friend_delta(friends, &friends_new).map_err(|e| Error::BadState {
                message: format!("failed to resolve friend delta: {e}"),
            })?;
        let friend_add = match friend_delta {
            FriendDelta::Add(it) => it,
            FriendDelta::Remove(info) => {
                tracing::warn!("pending_friends: friend conflict detected(remove): {info:?}");
                worker_write_event!(
                    WorkerEventType::Warn,
                    format!("friend conflict detected: lesser : {info:?}")
                );
                continue;
            }
            FriendDelta::Same => {
                worker_write_event!(
                    WorkerEventType::Warn,
                    format!("friend conflict detected: Same")
                );
                tracing::warn!("pending_friends: friends keep same, may triggered something");
                continue;
            }
        };
        ids.insert(job.essential.friend_code.clone(), friend_add.clone());

        *friends = friends_new;
        push_friend_info(&mut connection, &friend_add, job)?;
        job.essential.cursor_start = cursor.cast_signed() as i32;
        job.state = JobState::Pending {
            friend_info: friend_add,
            start_timestamp,
            current_length: 0,
        };
    }

    Ok(())
}

fn push_friend_info(
    connection: &mut Connection,
    friend_info: &FriendInfo,
    job: &Job,
) -> Result<(), Error> {
    let key = job_result_friend_info_redis_key(&job.get_stream_key_postfix());
    connection
        .lpush(
            &key,
            serde_json::to_string(friend_info).map_err(Error::JsonParse)?,
        )
        .map_err(Error::Redis)?;
    connection.ltrim(&key, 0, 1).map_err(Error::Redis)?;
    Ok(())
}
