use std::{collections::HashSet, env, hash::RandomState};

use axum::{Json, extract::Query, response::IntoResponse};
use base64::Engine;
use cuscuta_common::{
    api::xxxxxx::SongScore,
    db::{
        job::{
            SearchPositionResult, fetch_pending_tags, fetch_result, fetch_result_tags,
            search_position,
        },
        job_eta::fetch_unit_eta,
        redis::{job_index_redis_key, job_output_index_redis_key},
    },
};
use redis::{Client, TypedCommands};
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
        retries: usize,
        result: Vec<SongScore>,
    },
    SuccessPending {
        success: bool,
        pending: bool,
        progress: f64,
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

#[allow(clippy::cast_precision_loss)]
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
        let evidence_check_result = check_evidence(redis_client, &token)?;
        match evidence_check_result {
            EvidenceCheckResult::Pending {
                total_jobs,
                finished_jobs,
            } => {
                // TODO: 这个设计只能全量查询，若需增量查询，需要改动
                let percent = round_fixed(finished_jobs as f64 / total_jobs as f64, 2);
                let eta = if enable_eta {
                    match calc_eta_millis(redis_client, &token).await {
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
                    progress: percent,
                })
            }
            EvidenceCheckResult::Finished { retries } => {
                let result = fetch_result(redis_client, &token)
                    .map_err(|e| Error::RedisExtend(ErrorType::FailedScanRedis, e))?;
                Ok(QueryResult::SuccessFinished {
                    success: true,
                    pending: false,
                    retries,
                    result,
                })
            }
            EvidenceCheckResult::Failed => {
                Err(Error::BadRequest(ErrorType::BadRequestTokenCheckFailed))
            }
        }
    }

    match op(query).await {
        Ok(token) => {
            let status_code = match &token {
                QueryResult::SuccessFinished { .. } => StatusCode::OK,
                QueryResult::SuccessPending { .. } => StatusCode::PARTIAL_CONTENT,
                QueryResult::Failed { .. } => unreachable!("it's should not happen"),
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
    },
    Finished {
        retries: usize,
    },
    Failed,
}

fn check_evidence(redis_client: &Client, postfix: &str) -> Result<EvidenceCheckResult, Error> {
    let mut connection = redis_client
        .get_connection()
        .map_err(|e| Error::Redis(ErrorType::FailedCheckEvidenceRedis, e))?;
    let total_jobs_len = connection
        .llen(job_index_redis_key(postfix))
        .map_err(|e| Error::Redis(ErrorType::FailedCheckEvidenceRedis, e))?;
    if total_jobs_len == 0 {
        return Ok(EvidenceCheckResult::Failed);
    }
    let finished_jobs_len = connection
        .llen(job_output_index_redis_key(postfix))
        .map_err(|e| Error::Redis(ErrorType::FailedCheckEvidenceRedis, e))?;
    if total_jobs_len > finished_jobs_len {
        return Ok(EvidenceCheckResult::Pending {
            total_jobs: total_jobs_len,
            finished_jobs: finished_jobs_len,
        });
    }
    let finished_jobs_tags = fetch_result_tags(redis_client, postfix)
        .map_err(|e| Error::RedisExtend(ErrorType::FailedCheckEvidenceRedis, e))?;
    let job_id_set = finished_jobs_tags
        .into_iter()
        .map(|it| it.job_essential.job_uid)
        .collect::<HashSet<String, RandomState>>();
    Ok(EvidenceCheckResult::Finished {
        retries: total_jobs_len - job_id_set.len(),
    })
}

#[allow(clippy::cast_precision_loss)]
async fn calc_eta_millis(redis_client: &Client, postfix: &str) -> Result<Option<f64>, Error> {
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
    let pending_jobs = fetch_pending_tags(redis_client, postfix)
        .map_err(|e| Error::RedisExtend(ErrorType::FailedReadEtaRedis, e))?;
    let avg_job_eta = pending_jobs
        .iter()
        .map(|it| it.queue.segment.len())
        .sum::<usize>() as f64
        / pending_jobs.len() as f64;
    let positions = pending_jobs
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
