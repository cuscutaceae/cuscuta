use std::env;

use reqwest::{RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;

/// Github Api [DEPRECATED]
pub mod github;

/// xxxxxx Api
pub mod xxxxxx;

/// chilo Api
pub mod chilo;

/// xxxxxx Api with auto chilo
pub mod auto_chilo_xxxxxx;

/// Api调用可能引发的错误
///
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
}

/// 从URL中获取JSON数据
///
/// # Errors
/// - 当请求发送失败时，返回[`Error::Network`]
/// - 当返回值不为2xx时，返回[`Error::BadStatus`]
/// - 当Json反序列化失败时，返回[`Error::Decode`]
pub async fn fetch_json<T>(url: &str) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(Error::Network)?
        .error_for_status_with_response()
        .await
        .map_err(|(s, e)| Error::BadStatus {
            status_code: e.status().unwrap_or_else(StatusCode::default),
            message: s,
            extra_error_code: None,
        })?
        .json::<T>()
        .await
        .map_err(|e| Error::Decode {
            message: format!("failed to decode json: {e}"),
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

trait LoggingRequestBeforeSend
where
    Self: Sized,
{
    fn send_after_log(self) -> impl Future<Output = Result<Response, reqwest::Error>>;
}

impl LoggingRequestBeforeSend for RequestBuilder {
    async fn send_after_log(self) -> Result<Response, reqwest::Error> {
        let (client, request) = self.build_split();
        let request = request?;
        tracing::debug!("request_logging: {request:?}");
        client.execute(request).await
    }
}
