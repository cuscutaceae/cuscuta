use std::str::FromStr;

use axum::http::Uri;

use crate::{
    api::{
        Error,
        xxxxxx::{FriendListResult1, LoginResult, SongScore, XxxxxxUrl},
    },
    data::AppVersionData,
};

/// a marco that call xxxxxx api with chilo call
#[macro_export]
macro_rules! auto_chilo {
    ($fun:ident, $path: expr, $body: expr, $url: expr, $($tt:tt)*) => { async {
        #[allow(unused_imports)]
        use $crate::api::{Error, xxxxxx, chilo::{self, ChiloResult}};
        use chrono::Utc;
        use axum::http::StatusCode;
        let timestamp = Utc::now().timestamp_millis().to_string();
        let chilo_result = chilo::chilo_generate(
            &$url.chilo,
            &timestamp,
            $path,
            $body,
        ).await;
        tracing::debug!("xxxxxx_api_calling: {}: {} | {}", stringify!($fun), $path, $body);
        match chilo_result {
            Ok(chilo_result) => {
                match chilo_result {
                    ChiloResult::Success { value } => {
                        xxxxxx::$fun($url, $($tt)* &value).await
                    },
                    ChiloResult::Failed { message } => Err(Error::BadStatus {
                        status_code: StatusCode::INTERNAL_SERVER_ERROR,
                        message,
                        extra_error_code: None,
                    }),
                }
            },
            Err(e) => Err(e),
        }
    }};
}

type Result<T> = core::result::Result<T, Error>;

fn get_path(url: &str) -> String {
    let uri = Uri::from_str(url).expect("failed to parse uri");
    let path = uri
        .path_and_query()
        .map_or_else(|| uri.path(), |it| it.as_str());
    path.to_string()
}

/// 通过xxxxxx api登录
///
/// # Errors
/// 参见[`super::xxxxxx`]下同名函数
pub async fn api_login(
    url: &XxxxxxUrl,
    bundle_data: &AppVersionData,
    email: &str,
    password: &str,
) -> Result<LoginResult> {
    auto_chilo!(
        api_login,
        &get_path(&url.login),
        "grant_type=client_credentials",
        url,
        bundle_data,
        email,
        password,
    )
    .await
}

/// 通过xxxxxx api查询好友
///
/// # Errors
/// 参见[`super::xxxxxx`]下同名函数
pub async fn api_list_friend(
    url: &XxxxxxUrl,
    bundle_data: &AppVersionData,
    email: &str,
    user_id: &str,
    token: &str,
) -> Result<FriendListResult1> {
    auto_chilo!(
        api_list_friend,
        &get_path(&url.list_friend),
        "",
        url,
        bundle_data,
        email,
        user_id,
        token,
    )
    .await
}

/// 通过xxxxxx api添加好友
///
/// # Errors
/// 参见[`super::xxxxxx`]下同名函数
pub async fn api_add_friend(
    url: &XxxxxxUrl,
    bundle_data: &AppVersionData,
    email: &str,
    user_id: &str,
    token: &str,
    friend_code: &str,
) -> Result<FriendListResult1> {
    auto_chilo!(
        api_add_friend,
        &get_path(&url.add_friend),
        &format!("friend_code={friend_code}"),
        url,
        bundle_data,
        email,
        user_id,
        token,
        friend_code,
    )
    .await
}

/// 通过xxxxxx api删除好友
///
/// # Errors
/// 参见[`super::xxxxxx`]下同名函数
pub async fn api_delete_friend(
    url: &XxxxxxUrl,
    bundle_data: &AppVersionData,
    email: &str,
    user_id: &str,
    token: &str,
    friend_id: &str,
) -> Result<FriendListResult1> {
    auto_chilo!(
        api_delete_friend,
        &get_path(&url.delete_friend),
        &format!("friend_id={friend_id}"),
        url,
        bundle_data,
        email,
        user_id,
        token,
        friend_id,
    )
    .await
}

/// 通过xxxxxx api查询排行榜
///
/// # Errors
/// 参见[`super::xxxxxx`]下同名函数
#[allow(clippy::too_many_arguments)]
pub async fn api_get_rank_list(
    url: &XxxxxxUrl,
    bundle_data: &AppVersionData,
    email: &str,
    user_id: &str,
    token: &str,
    song_id: &str,
    difficulty: &str,
    start: &str,
    limit: &str,
) -> Result<Vec<SongScore>> {
    let query = format!(
        "song_id={}&difficulty={}&start={}&limit={}",
        urlencoding::encode(song_id),
        urlencoding::encode(difficulty),
        urlencoding::encode(start),
        urlencoding::encode(limit),
    );
    auto_chilo!(
        api_get_rank_list,
        &format!("{}?{query}", get_path(&url.get_rank)),
        "",
        url,
        bundle_data,
        email,
        user_id,
        token,
        song_id,
        difficulty,
        start,
        limit,
    )
    .await
}

/// 通过xxxxxx api查询通知
///
/// 本操作的返回将被丢弃
///
/// # Errors
/// 参见[`super::xxxxxx`]下同名函数
pub async fn api_get_notification(
    url: &XxxxxxUrl,
    bundle_data: &AppVersionData,
    email: &str,
    user_id: &str,
    token: &str,
) -> Result<()> {
    auto_chilo!(
        api_get_notification,
        &get_path(&url.get_notification),
        "",
        url,
        bundle_data,
        email,
        user_id,
        token,
    )
    .await
}

/// 通过xxxxxx api进行聚合查询
///
/// 本操作的返回将被丢弃
///
/// # Errors
/// 参见[`super::xxxxxx`]下同名函数
pub async fn api_compose_aggregate(
    url: &XxxxxxUrl,
    bundle_data: &AppVersionData,
    email: &str,
    user_id: &str,
    token: &str,
) -> Result<()> {
    auto_chilo!(
        api_compose_aggregate,
        &get_path(&url.compose),
        "",
        url,
        bundle_data,
        email,
        user_id,
        token,
    )
    .await
}
