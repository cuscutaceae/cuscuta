use cuscuta_common::{
    api::{
        self,
        xxxxxx::{FriendDelta, FriendInfo, auto::xxxxxx_safe_call_ex, calc_friend_delta},
    },
    data::BundleData,
    db::{
        account::AccountRow,
        job::{Job, JobFailure, JobFailureResuming, JobFailureType, JobState},
        redis::job_result_friend_info_redis_key,
    },
};
use redis::{Client, Connection, TypedCommands};
use reqwest::StatusCode;

use crate::{data::Config, worker::Error};

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
    let ids: Vec<_> = jobs
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
        let option_existing_ids = ids
            .iter()
            .find(|(code, _)| code == &job.essential.friend_code);
        if let Some((_, existing_friend_info)) = option_existing_ids {
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
        let result = xxxxxx_safe_call_ex(
            config.worker_max_retry_count,
            config.worker_exponential_backoff_base_millis,
            config.worker_exponential_backoff_multiplier,
            config.worker_exponential_backoff_max_delay_millis,
            |it| it != StatusCode::TOO_MANY_REQUESTS,
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
                if let api::Error::BadStatus(code) = &e {
                    if *code == 400 {
                        log::warn!(
                            "pending_friends: friend is already exist but cache is out-dated!"
                        );
                    } else {
                        log::warn!("pending_friends: failed to add friend: {e}: code: {code}");
                        job.state = JobState::Failed {
                            start_timestamp,
                            failure_info: JobFailure::new(
                                JobFailureType::FriendNotFound,
                                JobFailureResuming::Drop,
                            ),
                            friend_info: None,
                        };
                    }
                    continue;
                }
                log::warn!("pending_friends: unexpected error: {e}");
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
