use std::{collections::HashSet, time::Duration};

use cuscuta_common::{
    api::{self, xxxxxx::FriendInfo},
    data::BundleData,
    db::{
        account::AccountRow,
        job::{JobFailure, JobFailureResuming, JobFailureType},
        log::WorkerEventType,
    },
};
use reqwest::StatusCode;
use tokio::time::sleep;

use crate::{api_compat::xxxxxx_safe_call_ex_worker, data::Config, worker_write_event};

#[derive(Debug)]
enum FriendModifyError {
    Api(api::Error),
    Wait,
}

pub enum ExpectedModify {
    Add { friend_code: String },
    Remove { friend_id: i64 },
}

pub enum WaitForResultError {
    LoopAgain,
    Api(api::Error),
}

pub async fn try_modify_remote_friend(
    config: &Config,
    bundle_data: &BundleData,
    user_id: &str,
    token: &str,
    account_row: &AccountRow,
    expect_modify: ExpectedModify,
    cached_friend_list: &[FriendInfo],
) -> Result<Vec<FriendInfo>, JobFailure> {
    let expect_ids_when_remove = match &expect_modify {
        ExpectedModify::Add { .. } => None,
        ExpectedModify::Remove { friend_id } => Some(
            cached_friend_list
                .iter()
                .map(|it| it.user_id)
                .filter(|it| *it != *friend_id)
                .collect::<HashSet<_>>(),
        ),
    };
    let expects = expect_ids_when_remove.as_ref().map_or_else(
        || either::Either::Left(cached_friend_list.len()),
        either::Either::Right,
    );
    loop {
        let result = try_modify_friend(
            config,
            bundle_data,
            user_id,
            token,
            account_row,
            &expect_modify,
            expects,
        )
        .await;
        match result {
            Ok(result) => {
                return Ok(result);
            }
            Err(FriendModifyError::Api(e)) => {
                let failure_info = if let api::Error::BadStatus {
                    status_code,
                    extra_error_code,
                    ..
                } = &e
                {
                    if *status_code == 404 {
                        JobFailure::new(JobFailureType::FriendNotFound, JobFailureResuming::Drop)
                    } else {
                        JobFailure::new(
                            JobFailureType::XxxxxxApiError(status_code.as_u16(), *extra_error_code),
                            JobFailureResuming::Drop,
                        )
                    }
                } else {
                    JobFailure::new(
                        JobFailureType::ApiError(format!("{e:?}")),
                        JobFailureResuming::Drop,
                    )
                };
                worker_write_event!(
                    WorkerEventType::Warn,
                    format!("failed to modify friend: {e:?}",)
                );
                return Err(failure_info);
            }
            Err(FriendModifyError::Wait) => {
                worker_write_event!(WorkerEventType::Warn, "triggered friend modify waiting");
                match wait_for_result(
                    config,
                    bundle_data,
                    user_id,
                    token,
                    account_row,
                    cached_friend_list,
                    expects,
                )
                .await
                {
                    Ok(result) => return Ok(result),
                    Err(err) => match err {
                        WaitForResultError::LoopAgain => {}
                        WaitForResultError::Api(err) => {
                            return Err(JobFailure::new(
                                JobFailureType::ApiError(err.to_string()),
                                JobFailureResuming::Drop,
                            ));
                        }
                    },
                }
            }
        }
    }
}

async fn wait_for_result(
    config: &Config,
    bundle_data: &BundleData,
    user_id: &str,
    token: &str,
    account_row: &AccountRow,
    cached_friend_list: &[FriendInfo],
    expects: either::Either<usize, &HashSet<i64>>,
) -> Result<Vec<FriendInfo>, WaitForResultError> {
    let cached_previous_ids = cached_friend_list
        .iter()
        .map(|it| it.user_id)
        .collect::<HashSet<_>>();
    loop {
        // To reviewers: Due to the target's rate limiting strategy,
        //               blocking work queue is expected behavior here
        sleep(Duration::from_secs(
            config.worker_empty_friends_delay_time_secs,
        ))
        .await;
        let result = xxxxxx_safe_call_ex_worker(
            config,
            |it| it != StatusCode::TOO_MANY_REQUESTS,
            || {
                api::xxxxxx::api_list_friend(
                    bundle_data,
                    &account_row.account_email,
                    user_id,
                    token,
                )
            },
        )
        .await
        .map(|it| it.friends)
        .map_err(WaitForResultError::Api)?;
        match expects {
            sqlx::Either::Left(origin_length) => {
                if result.is_empty() {
                    continue;
                } else if result.len() != origin_length + 1 {
                    return Err(WaitForResultError::LoopAgain);
                }
                return Ok(result);
            }
            sqlx::Either::Right(expect_ids) => {
                let real_ids = result.iter().map(|it| it.user_id).collect::<HashSet<_>>();
                if &real_ids == expect_ids {
                    return Ok(result);
                } else if real_ids == cached_previous_ids {
                    return Err(WaitForResultError::LoopAgain);
                }
            }
        }
    }
}

async fn try_modify_friend(
    config: &Config,
    bundle_data: &BundleData,
    user_id: &str,
    token: &str,
    account_row: &AccountRow,
    expect_modify: &ExpectedModify,
    expects: either::Either<usize, &HashSet<i64>>,
) -> Result<Vec<FriendInfo>, FriendModifyError> {
    let result = match &expect_modify {
        ExpectedModify::Add { friend_code } => {
            xxxxxx_safe_call_ex_worker(
                config,
                |it| it != StatusCode::TOO_MANY_REQUESTS,
                || {
                    api::xxxxxx::api_add_friend(
                        bundle_data,
                        &account_row.account_email,
                        user_id,
                        token,
                        friend_code,
                    )
                },
            )
            .await
        }
        ExpectedModify::Remove { friend_id } => {
            let friend_id_str = friend_id.to_string();
            xxxxxx_safe_call_ex_worker(
                config,
                |it| it != StatusCode::TOO_MANY_REQUESTS,
                || {
                    api::xxxxxx::api_delete_friend(
                        bundle_data,
                        &account_row.account_email,
                        user_id,
                        token,
                        &friend_id_str,
                    )
                },
            )
            .await
        }
    };
    match result {
        Err(e) => {
            if let api::Error::BadStatus {
                status_code,
                extra_error_code,
                message,
            } = &e
            {
                tracing::warn!(
                    "try_modify_friends: failed to call api: HTTP {status_code} {message}"
                );
                worker_write_event!(
                    WorkerEventType::Warn,
                    format!(
                        "failed to modify friend: HTTP {status_code}: {extra_error_code:?}: {message}"
                    )
                );
            } else {
                tracing::warn!("try_modify_friends: unexpected error: {e}");
            }
            Err(FriendModifyError::Api(e))
        }
        Ok(friend_list) => {
            let success = match expects {
                either::Either::Right(expect_ids) => {
                    let real_ids = friend_list
                        .friends
                        .iter()
                        .map(|it| it.user_id)
                        .collect::<HashSet<_>>();
                    let success = expect_ids == &real_ids;
                    if !success {
                        tracing::warn!(
                            "try_modify_friends: returning friends is mismatched, waiting"
                        );
                    }
                    success
                }
                either::Either::Left(origin_len) => {
                    let success = friend_list.friends.len() == origin_len + 1;
                    if !success {
                        tracing::warn!("try_modify_friends: returning friends is empty, waiting");
                    }
                    success
                }
            };
            if success {
                return Ok(friend_list.friends);
            }
            Err(FriendModifyError::Wait)
        }
    }
}
