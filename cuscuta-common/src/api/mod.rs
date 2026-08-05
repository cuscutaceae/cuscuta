use std::env;

use reqwest::{Response, StatusCode};

/// Github Api
pub mod github;

/// xxxxxx Api
pub mod xxxxxx;

/// chilo Api
pub mod chilo;

/// Api调用可能引发的错误
///
/// 注：这里的错误处理可能很脏，因为这个错误类型包含了过于特化的[`Self::TooManyRetries`]
/// 以及并非所有Api系函数均支持的[`Self::ApiError`]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `reqwest`客户端初始化失败
    #[error("failed to setup client: {0}")]
    ClientSetup(reqwest::Error),

    /// 网络错误
    #[error("failed to send request: {0}")]
    Network(reqwest::Error),

    /// Api的`HTTP`返回码不为2xx或1xx
    #[error("bad return status: HTTP {status_code} {extra_error_code:?}: {message}")]
    BadStatus {
        /// 错误码
        status_code: StatusCode,

        /// 错误描述
        message: String,

        /// Api错误码（如果有）
        extra_error_code: Option<i64>,
    },

    /// Json反序列化失败
    #[error("failed to decode response: {message}")]
    Decode {
        /// 错误描述
        message: String,
    },

    /// 环境变量未配置或配置无效
    #[error("failed to read env::var: {error}:{message}")]
    Env {
        /// 环境变量错误
        error: env::VarError,

        /// 错误描述
        message: String,
    },
}

fn try_get_env_var(var: &str) -> Result<String, Error> {
    env::var(var).map_err(|error| Error::Env {
        error,
        message: var.to_string(),
    })
}

trait ErrorForStatusWithResponse
where
    Self: Sized,
{
    fn error_for_status_with_response(
        self,
    ) -> impl Future<Output = Result<Self, (String, reqwest::Error)>>;
}

impl ErrorForStatusWithResponse for Response {
    async fn error_for_status_with_response(self) -> Result<Self, (String, reqwest::Error)> {
        match self.error_for_status_ref() {
            Ok(_) => Ok(self),
            Err(e) => Err((
                self.text()
                    .await
                    .unwrap_or_else(|e| format!("[FAILED TO GET BODY] {e}")),
                e,
            )),
        }
    }
}
