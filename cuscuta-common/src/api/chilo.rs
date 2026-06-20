use serde::Deserialize;

use crate::api::{Error, try_get_env_var};

/// `chilo`执行的结果
#[derive(Deserialize)]
#[serde(untagged)]
pub enum ChiloResult {
    /// 执行成功
    Success {
        /// 获取到的Challenge值
        value: String,
    },

    /// 执行失败
    Failed {
        /// 失败原因
        message: String,
    },
}

type Result<T> = core::result::Result<T, Error>;

/// 调用`chilo`接口，返回Challenge值
///
/// # Errors
/// - 当环境变量配置无效时，返回[`Error::Env`]
/// - 当`reqwest`客户端初始化失败时，返回[`Error::ClientSetup`]
/// - 当请求发送失败时，返回[`Error::Network`]
/// - 当Json反序列化失败时，返回[`Error::Decode`]
pub async fn chilo_generate(timestamp: &str, path: &str, kind: &str) -> Result<ChiloResult> {
    reqwest::Client::builder()
        .user_agent("curl/7.88.1")
        .build()
        .map_err(Error::ClientSetup)?
        .get(try_get_env_var("API_CHILO")?)
        .query(&[("timestamp", &timestamp), ("path", &path), ("kind", &kind)])
        .send()
        .await
        .map_err(Error::Network)?
        .json::<ChiloResult>()
        .await
        .map_err(|e| Error::Decode(e.to_string()))
}
