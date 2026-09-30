use std::{collections::HashMap, str::FromStr, sync::LazyLock};

use axum::http::Uri;

use crate::{
    api::{
        Error, try_get_env_var,
        xxxxxx::{FriendListResult1, SongScore},
    },
    data::BundleData,
};

/// kind、path、body和函数名之间的映射
pub static MAP: LazyLock<HashMap<&str, &str>> = LazyLock::new(|| {
    HashMap::from([
        ("api_login", "login"),
        ("api_list_friend", "friend_list"),
        ("api_add_friend", "friend_add"),
        ("api_delete_friend", "friend_delete"),
        ("api_get_rank_list", "friend_rank"),
        ("api_get_notification", "notification"),
        ("api_compose_aggregate", "compose"),
    ])
});

/// a marco that call xxxxxx api with chilo call
#[macro_export]
macro_rules! auto_chilo {
    ($fun:ident, $path: expr, $body: expr, $($tt:tt)*) => { async {
        #[allow(unused_imports)]
        use $crate::api::{auto_chilo_xxxxxx::MAP, Error, xxxxxx, chilo::{self, ChiloResult}};
        use chrono::Utc;
        use axum::http::StatusCode;
        let kind = MAP[stringify!($fun)];
        let timestamp = Utc::now().timestamp_millis().to_string();
        let chilo_result = chilo::chilo_generate(
            &timestamp,
            $path,
            $body,
            kind
        ).await;
        tracing::debug!("xxxxxx_api_calling: {}: {} | {}", stringify!($fun), $path, $body);
        match chilo_result {
            Ok(chilo_result) => {
                match chilo_result {
                    ChiloResult::Success { value } => {
                        xxxxxx::$fun($($tt)* &value).await
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

fn get_path(env_var: &str) -> Result<String> {
    let env = try_get_env_var(env_var)?;
    let uri = Uri::from_str(&env).expect("failed to parse uri");
    let path = uri
        .path_and_query()
        .map_or_else(|| uri.path(), |it| it.as_str());
    Ok(path.to_string())
}

/// 通过xxxxxx api查询好友
///
/// # Errors
/// 参见[`super::xxxxxx`]下同名函数
pub async fn api_list_friend(
    bundle_data: &BundleData,
    email: &str,
    user_id: &str,
    token: &str,
) -> Result<FriendListResult1> {
    auto_chilo!(
        api_list_friend,
        &get_path("API_LIST_FRIENDS")?,
        "",
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
    bundle_data: &BundleData,
    email: &str,
    user_id: &str,
    token: &str,
    friend_code: &str,
) -> Result<FriendListResult1> {
    auto_chilo!(
        api_add_friend,
        &get_path("API_ADD_FRIENDS")?,
        &format!("friend_code={friend_code}"),
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
    bundle_data: &BundleData,
    email: &str,
    user_id: &str,
    token: &str,
    friend_id: &str,
) -> Result<FriendListResult1> {
    auto_chilo!(
        api_delete_friend,
        &get_path("API_DELETE_FRIENDS")?,
        &format!("friend_id={friend_id}"),
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
    bundle_data: &BundleData,
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
        &format!("{}?{query}", get_path("API_GET_RANK")?),
        "",
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
    bundle_data: &BundleData,
    email: &str,
    user_id: &str,
    token: &str,
) -> Result<()> {
    auto_chilo!(
        api_get_notification,
        &get_path("API_NOTIFICATION")?,
        "",
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
    bundle_data: &BundleData,
    email: &str,
    user_id: &str,
    token: &str,
) -> Result<()> {
    auto_chilo!(
        api_compose_aggregate,
        &get_path("API_COMPOSE_AGGREGATE")?,
        "",
        bundle_data,
        email,
        user_id,
        token,
    )
    .await
}
