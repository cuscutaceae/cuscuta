use std::env;

use axum::{Json, extract::Query, response::IntoResponse};
use base64::Engine;
use cuscuta_common::{
    api::xxxxxx::{FriendInfo, SongScore},
    db::{
        job::{
            eta::fetch_unit_eta,
            fetch::{SearchPositionResult, fetch_result, search_position},
            track::{JobTrackQueueStatus, JobTrackTag, fetch_all_job_track_tag},
        },
        redis::{job_result_friend_info_redis_key, job_result_value_redis_key},
    },
};
use redis::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    db::{account::count_active_account, postgresql::try_open_transaction, redis::REDIS_CLIENT},
    endpoints::{Error, ErrorType, round_fixed},
};

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum QueryResult {
    SuccessFinished {
        success: bool,
        pending: bool,
        friend_info: Option<FriendInfo>,
        result: Vec<SongScore>,
    },
    SuccessPending {
        success: bool,
        pending: bool,
        total_jobs: usize,
        finished_jobs: usize,
        pending_jobs: usize,
        queueing_jobs: usize,
        friend_info: Option<FriendInfo>,
        eta: Option<f64>,
    },
    Failed {
        success: bool,
        code: i64,
        message: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct QueryQuery {
    token: String,
}

pub async fn query(Query(query): Query<QueryQuery>) -> impl IntoResponse {
    async fn op(query: QueryQuery) -> Result<QueryResult, Error> {
        let redis_client = REDIS_CLIENT
            .get()
            .ok_or(Error::NotReady(ErrorType::RedisNotReady))?;
        let token = String::from_utf8(
            base64::prelude::BASE64_URL_SAFE
                .decode(query.token)
                .map_err(|_| Error::BadRequest(ErrorType::BadRequestBase64))?,
        )
        .map_err(|_| Error::BadRequest(ErrorType::BadRequestBase64))?;
        let enable_eta = env::var("ETA_ENABLE")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);
        let (evidence_check_result, temp_track_tags) = check_evidence(redis_client, &token);
        let friend_info =
            fetch_result::<FriendInfo>(redis_client, &job_result_friend_info_redis_key(&token))
                .map_or(None, |it| it.into_iter().next());
        match evidence_check_result {
            EvidenceCheckResult::Pending {
                total_jobs,
                finished_jobs,
                pending_jobs,
                queueing_jobs,
            } => {
                // TODO: 这个设计只能全量查询，若需增量查询，需要改动
                let eta = if let Some(job_track_tags) = temp_track_tags
                    && enable_eta
                {
                    match calc_eta_millis(redis_client, &token, &job_track_tags).await {
                        Ok(v) => v.map(|it| round_fixed(it / 1000.0, 2)),
                        Err(e) => {
                            log::warn!("eta: failed to calc eta: {e}");
                            None
                        }
                    }
                } else {
                    None
                };
                Ok(QueryResult::SuccessPending {
                    success: true,
                    pending: true,
                    eta,
                    friend_info,
                    total_jobs,
                    finished_jobs,
                    pending_jobs,
                    queueing_jobs,
                })
            }
            EvidenceCheckResult::Finished => {
                let result = fetch_result(redis_client, &job_result_value_redis_key(&token))
                    .map_err(|e| Error::RedisExtend(ErrorType::FailedScanRedis, e))?;
                Ok(QueryResult::SuccessFinished {
                    success: true,
                    pending: false,
                    friend_info,
                    result,
                })
            }
            EvidenceCheckResult::JobFailed { code, message, .. } => Ok(QueryResult::Failed {
                success: false,
                code,
                message,
            }),
            EvidenceCheckResult::CheckFailed => {
                Err(Error::BadRequest(ErrorType::BadRequestTokenCheckFailed))
            }
        }
    }

    match op(query).await {
        Ok(token) => {
            let status_code = match &token {
                QueryResult::SuccessFinished { .. } => StatusCode::OK,
                QueryResult::SuccessPending { .. } => StatusCode::PARTIAL_CONTENT,
                QueryResult::Failed { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status_code, Json(token))
        }
        Err(e) => {
            log::warn!("endpoint enqueue failed: {e}");
            (
                e.get_status_code(),
                Json(QueryResult::Failed {
                    success: false,
                    code: e.get_error_type() as i64,
                    message: format!("{e}"),
                }),
            )
        }
    }
}

enum EvidenceCheckResult {
    Pending {
        total_jobs: usize,
        finished_jobs: usize,
        pending_jobs: usize,
        queueing_jobs: usize,
    },
    Finished,
    JobFailed {
        code: i64,
        message: String,
    },
    CheckFailed,
}

fn check_evidence(
    redis_client: &Client,
    postfix: &str,
) -> (EvidenceCheckResult, Option<Vec<JobTrackTag>>) {
    let Ok(job_tracks) = fetch_all_job_track_tag(redis_client, postfix) else {
        return (EvidenceCheckResult::CheckFailed, None);
    };
    if job_tracks
        .iter()
        .all(|it| it.status == JobTrackQueueStatus::Success)
    {
        return (EvidenceCheckResult::Finished, None);
    }
    if job_tracks
        .iter()
        .any(|it| it.status == JobTrackQueueStatus::Failed)
    {
        return if let Some(first_failed) = job_tracks
            .iter()
            .find(|it| matches!(it.status, JobTrackQueueStatus::Failed))
            .expect("first element not found, this should not happen")
            .failures
            .first()
        {
            let failure_type = first_failed.fail_type.get_repr();
            (
                EvidenceCheckResult::JobFailed {
                    code: failure_type.into(),
                    message: format!("{:?}", first_failed.fail_type),
                },
                Some(job_tracks),
            )
        } else {
            (
                EvidenceCheckResult::JobFailed {
                    code: -998,
                    message: "no further information... ask nofyso about it may help...?"
                        .to_owned(),
                },
                Some(job_tracks),
            )
        };
    }
    let pending_jobs = job_tracks
        .iter()
        .filter(|it| matches!(it.status, JobTrackQueueStatus::Pending))
        .count();
    let queueing_jobs = job_tracks
        .iter()
        .filter(|it| matches!(it.status, JobTrackQueueStatus::Queueing))
        .count();
    let finished_jobs = job_tracks
        .iter()
        .filter(|it| matches!(it.status, JobTrackQueueStatus::Success))
        .count();
    (
        EvidenceCheckResult::Pending {
            total_jobs: job_tracks.len(),
            finished_jobs,
            pending_jobs,
            queueing_jobs,
        },
        Some(job_tracks),
    )
}

#[allow(clippy::cast_precision_loss)]
async fn calc_eta_millis(
    redis_client: &Client,
    _postfix: &str,
    job_track_tags: &[JobTrackTag],
) -> Result<Option<f64>, Error> {
    let transaction = try_open_transaction()
        .await
        .map_err(|e| Error::DbExtend(ErrorType::FailedTransactionOpenDb, e))?;
    let active_account_count = count_active_account(transaction)
        .await
        .map_err(|e| Error::Db(ErrorType::FailedCountDb, e))?;
    let eta_record_trim = env::var("ETA_RECORD_TRIM")
        .map_err(|_| ())
        .and_then(|it| it.parse::<usize>().map_err(|_| ()))
        .unwrap_or(15);
    let eta_search_limit = env::var("ETA_SEARCH_LIMIT")
        .map_err(|_| ())
        .and_then(|it| it.parse::<usize>().map_err(|_| ()))
        .unwrap_or(10);
    let Some(estimated_unit_eta) = fetch_unit_eta(redis_client, eta_record_trim)
        .map_err(|e| Error::RedisExtend(ErrorType::FailedReadEtaRedis, e))?
    else {
        return Ok(None);
    };
    let avg_job_eta = job_track_tags
        .iter()
        .map(|it| it.queue.segment.len())
        .sum::<usize>() as f64
        / job_track_tags.len() as f64;
    let positions = job_track_tags
        .iter()
        .filter_map(|it| {
            let x = search_position(
                redis_client,
                eta_search_limit,
                &it.job_essential.job_uid,
                &it.queue.name,
            );
            if let Err(e) = &x {
                log::warn!("eta_debug: failed to search_position: {e}");
            }
            x.ok()
        })
        .filter(|it| it != &SearchPositionResult::QueueingNotFound)
        .collect::<Vec<_>>();
    if positions.is_empty() {
        return Ok(None);
    }
    if positions
        .iter()
        .all(|it| it == &SearchPositionResult::Pending)
    {
        return Ok(Some(estimated_unit_eta * avg_job_eta));
    }
    let max_found = positions
        .iter()
        .filter_map(|it| match it {
            SearchPositionResult::QueueingFound(x) => Some(x),
            _ => None,
        })
        .max()
        .copied();
    let found_counts = positions
        .iter()
        .filter(|it| matches!(it, SearchPositionResult::QueueingFound(_)))
        .count();
    let estimated_multiply = ((found_counts as f64) / (active_account_count as f64))
        .ceil()
        .max(1.0);
    Ok(max_found.map(|it| {
        let add = usize::from(
            positions
                .iter()
                .any(|it| it == &SearchPositionResult::Pending),
        );
        estimated_unit_eta * (it + 1 + add) as f64 * avg_job_eta * estimated_multiply
    }))
}
