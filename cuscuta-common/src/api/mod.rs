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
/// 注：这里的错误处理可能很脏，因为这个错误类型包含了过于特意化的[`Self::TooManyRetries`]
/// 以及[`Self::`]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `reqwest`客户端初始化失败
    #[error("failed to setup client: {0}")]
    ClientSetup(reqwest::Error),

    /// 网络错误
    #[error("failed to send request: {0}")]
    Network(reqwest::Error),

    /// Api的`HTTP`返回码不为2xx或1xx
    #[error("bad return status: {0}: {1}")]
    BadStatus(StatusCode, String),

    /// Json反序列化失败
    #[error("failed to decode response: {0}")]
    Decode(String),

    /// 环境变量未配置或配置无效
    #[error("failed to read env::var: {0}:{1}")]
    Env(env::VarError, String),

    /// 重试次数过多
    #[error("too many retries: inner: {0}")]
    TooManyRetries(String),

    /// 具有更多信息的Api错误
    #[error("bad api return: {0}: {1}")]
    ApiError(i64, String),
}

fn try_get_env_var(var: &str) -> Result<String, Error> {
    env::var(var).map_err(|e| Error::Env(e, var.to_string()))
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
