use axum::{Json, response::IntoResponse};
use cuscuta_common::{
    db::log::{WorkerStatus, status::search_worker_status},
    quick_fetch::QuickFetch,
};
use reqwest::StatusCode;
use serde::Serialize;

use crate::{
    data::CONFIG,
    db::redis::REDIS_CLIENT,
    endpoints::{Error, ErrorType},
};

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum StatResult<'a> {
    Success {
        success: bool,
        worker_status: Vec<WorkerStatus<'a>>,
    },
    Failed {
        success: bool,
        code: i64,
        message: String,
    },
}

pub async fn stat() -> impl IntoResponse {
    fn op<'a>() -> Result<StatResult<'a>, Error> {
        let redis_client = REDIS_CLIENT
            .get()
            .ok_or(Error::NotReady(ErrorType::RedisNotReady))?;
        let config = CONFIG
            .try_read(std::clone::Clone::clone)
            .map_err(|_| Error::NotReady(ErrorType::ConfigNotReady))?;
        let worker_status = search_worker_status(redis_client)
            .map_err(|e| Error::RedisExtend(ErrorType::FailedScanRedis, e))?;
        if !config.enable_stat {
            return Err(Error::BadRequest(ErrorType::BadRequestNotEnabled));
        }
        Ok(StatResult::Success {
            success: true,
            worker_status: worker_status.into_iter().map(|it| it.1).collect(),
        })
    }
    match op() {
        Ok(x) => (StatusCode::OK, Json(x)),
        Err(e) => (
            e.get_status_code(),
            Json(StatResult::Failed {
                success: false,
                code: e.get_error_type() as i64,
                message: format!("{e}"),
            }),
        ),
    }
}
