use std::fmt::Display;

use reqwest::StatusCode;

pub mod enqueue;
pub mod query;

#[derive(Debug, Clone, Copy)]
enum ErrorType {
    RedisNotReady = -101,
    ConfigNotReady = -102,
    SongListNotReady = -103,

    FailedTransactionOpenDb = -201,
    FailedCountDb = -202,

    FailedScanRedis = -301,
    FailedEnqueueRedis = -302,
    FailedReadEtaRedis = -303,
    FailedCheckEvidenceRedis = -304,

    InternalNoWorker = -400,

    BadRequestBase64 = -500,
    BadRequestTokenCheckFailed = -501,
}

impl Display for ErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}", *self as i64))
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("server is not ready:{0}")]
    NotReady(ErrorType),
    #[error("service unavailable: 0x010:{0}")]
    Db(ErrorType, sqlx::Error),
    #[error("service unavailable: 0x011:{0}")]
    DbExtend(ErrorType, cuscuta_common::db::postgresql::Error),
    #[error("service unavailable: 0x020:{0}")]
    Redis(ErrorType, redis::RedisError),
    #[error("service unavailable: 0x021:{0}")]
    RedisExtend(ErrorType, cuscuta_common::db::redis::Error),
    #[error("service unavailable: 0x000:{0}")]
    Internal(ErrorType),
    #[error("bad request:{0}")]
    BadRequest(ErrorType),
}

impl Error {
    fn get_error_type(&self) -> ErrorType {
        match self {
            Error::NotReady(error_type)
            | Error::Db(error_type, _)
            | Error::DbExtend(error_type, _)
            | Error::Redis(error_type, _)
            | Error::RedisExtend(error_type, _)
            | Error::Internal(error_type)
            | Error::BadRequest(error_type) => *error_type,
        }
    }

    fn get_status_code(&self) -> StatusCode {
        match self {
            Error::NotReady(_)
            | Error::Db(_, _)
            | Error::DbExtend(_, _)
            | Error::Redis(_, _)
            | Error::RedisExtend(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Internal(_) => StatusCode::IM_A_TEAPOT,
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
        }
    }
}
