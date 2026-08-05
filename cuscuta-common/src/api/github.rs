use base64::Engine;
use reqwest::StatusCode;
use serde::{Deserialize, de::DeserializeOwned};

use crate::api::{Error, ErrorForStatusWithResponse};

#[derive(Debug, Deserialize)]
struct GitHubFileInternal {
    content: String,
    download_url: String,
}

type Result<T> = core::result::Result<T, Error>;

/// 调用Github Api，从Github上拉取资源
///
/// # Errors
/// - 当环境变量配置无效时，返回[`Error::Env`]
/// - 当`reqwest`客户端初始化失败时，返回[`Error::ClientSetup`]
/// - 当请求发送失败时，返回[`Error::Network`]
/// - 当返回值不为2xx时，返回[`Error::BadStatus`]
/// - 当Json反序列化失败时，返回[`Error::Decode`]
pub async fn fetch_github_resource<T>(repo: &str, path: &str, token: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let file_object = reqwest::Client::builder()
        .user_agent("curl/7.88.1")
        .build()
        .map_err(Error::ClientSetup)?
        .get(format!(
            "https://api.github.com/repos/{repo}/contents/{path}"
        ))
        .header("Accept", "application/vnd.github.object")
        .header("Authorization", format!("Bearer {token}"))
        .header("X-GitHub-Api-Version", "2026-03-10")
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
        .json::<GitHubFileInternal>()
        .await
        .map_err(|e| Error::Decode {
            message: format!("phase1: {e}"),
        })?;
    if file_object.content.is_empty() {
        reqwest::Client::builder()
            .user_agent("curl/7.88.1")
            .build()
            .map_err(Error::ClientSetup)?
            .get(file_object.download_url)
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
                message: format!("phase b_1: {e}"),
            })
    } else {
        base64::prelude::BASE64_STANDARD
            .decode(file_object.content.replace('\n', ""))
            .map_err(|e| Error::Decode {
                message: format!("phase a_2: {e}"),
            })
            .map(|it| {
                String::from_utf8(it).map_err(|e| Error::Decode {
                    message: format!("phase a_3: {e}"),
                })
            })?
            .map(|it| {
                serde_json::from_str::<T>(&it).map_err(|e| Error::Decode {
                    message: format!("phase a_4: {e}"),
                })
            })?
    }
}
