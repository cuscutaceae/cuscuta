use axum::{Json, extract::Query, response::IntoResponse};
use base64::Engine;
use cuscuta_common::{
    api::xxxxxx::SongScore,
    db::job::{JobTag, fetch_result, scan_fragment},
    quick_fetch::QuickFetch,
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    data::SONG_LIST,
    db::redis::REDIS_CLIENT,
    endpoints::{Error, ErrorType},
};

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn round_fixed(v: f64, n: u32) -> f64 {
    let i = 10_usize.pow(n) as f64;
    let x = v * i;
    if v > 0_f64 {
        f64::from(x.round() as u32) / i
    } else {
        let mr = x.trunc();
        let mf = x.fract();
        if mf.abs() >= 0.5 {
            return (mr + 1_f64) / i;
        }
        mr / i
    }
}

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
    fn op(query: QueryQuery) -> Result<QueryResult, Error> {
        let redis_client = REDIS_CLIENT
            .get()
            .ok_or(Error::NotReady(ErrorType::RedisNotReady))?;
        let song_list_len = SONG_LIST
            .try_read(std::vec::Vec::len)
            .map_err(|_| Error::NotReady(ErrorType::SongListNotReady))?;
        let token = base64::prelude::BASE64_URL_SAFE
            .decode(query.token)
            .map_err(|_| Error::BadRequest(ErrorType::BadRequestBase64))?;
        let token =
            String::from_utf8(token).map_err(|_| Error::BadRequest(ErrorType::BadRequestBase64))?;
        let token: Vec<_> = token.split('|').collect();
        if token.len() != 2 {
            return Err(Error::BadRequest(ErrorType::BadRequestBase64));
        }
        let key = token[0];
        let evidence: Vec<_> = token[1].split(',').collect();
        let fragments = scan_fragment(redis_client, key)
            .map_err(|e| Error::RedisExtend(ErrorType::FailedScanRedis, e))?;
        let fragment_length: usize = fragments
            .iter()
            .map(|it| it.job_essential.cursor_length.cast_unsigned() as usize)
            .sum();
        // TODO: 这个设计只能全量查询，若需增量查询，需要改动
        if fragment_length < song_list_len {
            let percent = round_fixed(fragment_length as f64 / song_list_len as f64, 2);
            return Ok(QueryResult::SuccessPending {
                success: true,
                pending: true,
                progress: percent,
            });
        }
        let EvidenceCheckResult { invalid, retries } = check_evidence(&evidence, &fragments)?;
        if invalid != 0 {
            return Err(Error::BadRequest(ErrorType::BadRequestTokenCheckFailed));
        }
        let result = fetch_result(redis_client, key)
            .map_err(|e| Error::RedisExtend(ErrorType::FailedScanRedis, e))?;
        Ok(QueryResult::SuccessFinished {
            success: true,
            pending: false,
            retries,
            result,
        })
    }

    match op(query) {
        Ok(token) => {
            let status_code = match &token {
                QueryResult::SuccessFinished { .. } => StatusCode::OK,
                QueryResult::SuccessPending { .. } => StatusCode::PARTIAL_CONTENT,
                QueryResult::Failed { .. } => panic!("it's should not happen"),
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

struct EvidenceCheckResult {
    invalid: usize,
    retries: usize,
}

fn check_evidence(evidence: &[&str], fragments: &[JobTag]) -> Result<EvidenceCheckResult, Error> {
    let new_evidence: Result<Vec<_>, _> = evidence
        .iter()
        .map(|it| {
            let arr: Vec<_> = it.split('_').collect();
            if arr.len() == 2 {
                Ok((arr[0].to_string(), arr[1].to_string()))
            } else {
                Err(Error::BadRequest(ErrorType::BadRequestBase64))
            }
        })
        .collect();
    let new_evidence = new_evidence?;
    let (invalid, retries) =
        new_evidence
            .iter()
            .fold((0usize, 0usize), |(invalid, retries), (first_id, uid)| {
                let find = fragments
                    .iter()
                    .find(|frag| &frag.job_essential.job_uid == uid);
                if let Some(tag) = find {
                    if &tag.job_last_id == first_id {
                        (invalid, retries)
                    } else {
                        (invalid, retries + 1)
                    }
                } else {
                    (invalid + 1, retries)
                }
            });
    Ok(EvidenceCheckResult { invalid, retries })
}
