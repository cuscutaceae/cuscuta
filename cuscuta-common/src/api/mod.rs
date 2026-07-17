use std::env;

use reqwest::StatusCode;

/// Github Api
pub mod github;

/// xxxxxx Api
pub mod xxxxxx;

/// chilo Api
pub mod chilo;

/// Api调用可能引发的错误
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `reqwest`客户端初始化失败
    #[error("failed to setup client: {0}")]
    ClientSetup(reqwest::Error),

    /// 网络错误
    #[error("failed to send request: {0}")]
    Network(reqwest::Error),

    /// Api的`HTTP`返回码不为2xx或1xx
    #[error("bad return status: {0}")]
    BadStatus(StatusCode),

    /// Json反序列化失败
    #[error("failed to decode response: {0}")]
    Decode(String),

    /// 环境变量未配置或配置无效
    #[error("failed to read env::var: {0}:{1}")]
    Env(env::VarError, String),

    /// 重试次数过多
    #[error("too many retries")]
    TooManyRetries,
}

fn try_get_env_var(var: &str) -> Result<String, Error> {
    env::var(var).map_err(|e| Error::Env(e, var.to_string()))
}
