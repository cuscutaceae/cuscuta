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

    InternalNoWorker = -400,

    BadRequestBase64 = -500,
    BadRequestTokenCheckFailed = -501,
    BadRequestFriendCode = -502,
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
    const fn get_error_type(&self) -> ErrorType {
        match self {
            Self::NotReady(error_type)
            | Self::Db(error_type, _)
            | Self::DbExtend(error_type, _)
            | Self::Redis(error_type, _)
            | Self::RedisExtend(error_type, _)
            | Self::Internal(error_type)
            | Self::BadRequest(error_type) => *error_type,
        }
    }

    const fn get_status_code(&self) -> StatusCode {
        match self {
            Self::NotReady(_)
            | Self::Db(_, _)
            | Self::DbExtend(_, _)
            | Self::Redis(_, _)
            | Self::RedisExtend(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Internal(_) => StatusCode::IM_A_TEAPOT,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
        }
    }
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn round_fixed(v: f64, n: u32) -> f64 {
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
