use std::collections::HashSet;
use std::fmt::Write;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    api::{Error, try_get_env_var},
    data::BundleData,
};

type Result<T> = core::result::Result<T, Error>;

/// xxxxxx api登录成功时的适配数据模型
#[derive(Debug, Deserialize, Clone)]
pub struct LoginResult {
    /// 用户id
    pub user_id: i64,

    /// 访问密钥
    pub access_token: String,

    /// 密钥类型，通常为`Bearer`
    pub token_type: String,
}

/// xxxxxx api好友变更时的适配数据模型（顶层）
#[derive(Debug, Deserialize, Clone)]
pub struct FriendListResult {
    /// 值
    pub value: FriendListResult1,
}

/// xxxxxx api好友变更时的适配数据模型（第二层）
#[derive(Debug, Deserialize, Clone)]
pub struct FriendListResult1 {
    /// 好友信息
    pub friends: Vec<FriendInfo>,
}

/// xxxxxx api好友信息的适配数据模型
#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct FriendInfo {
    /// 好友游戏名
    pub name: String,

    /// 好友用户id
    pub user_id: i64,

    /// 好友的评级（实际显示评级为`rating/10.0`）
    pub rating: i64,

    /// 好友设置的搭档
    pub character: i64,
}

/// xxxxxx api曲目成绩的适配数据模型（顶层）
#[derive(Debug, Serialize, Deserialize)]
pub struct SongScoreResult {
    ///曲目信息
    pub value: Vec<SongScore>,
}

/// xxxxxx api曲目成绩的适配数据模型
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SongScore {
    /// 曲目id
    pub song_id: String,

    /// 玩家的id
    pub user_id: i64,

    /// 难度等级（如0、1、2）
    pub difficulty: i64,

    /// 游玩分数
    pub score: i64,

    /// 游玩分数距最高分的差距
    pub score_below_max: i64,

    /// 大P数目
    pub shiny_perfect_count: i64,

    /// 小P数目
    pub perfect_count: i64,

    /// Far数目
    pub near_count: i64,

    /// Lost数目
    pub miss_count: i64,

    /// 通关类型
    pub clear_type: i64,

    /// 最高纪录的通关类型
    pub best_clear_type: i64,

    /// 通关时搭档血量
    pub health: i64,

    /// 游玩时间戳
    pub time_played: i64,

    /// 玩家名字
    #[serde(rename = "name")]
    pub player_name: String,
}

/// 通过xxxxxx api登录
///
/// # Errors
/// - 当环境变量配置无效时，返回[`Error::Env`]
/// - 当`reqwest`客户端初始化失败时，返回[`Error::ClientSetup`]
/// - 当请求发送失败时，返回[`Error::Network`]
/// - 当返回值不为2xx时，返回[`Error::BadStatus`]
/// - 当Json反序列化失败时，返回[`Error::Decode`]
pub async fn api_login(
    bundle_data: &BundleData,
    email: String,
    password: String,
    random_challenge: String,
) -> Result<LoginResult> {
    reqwest::Client::builder()
        .user_agent("curl/7.88.1")
        .build()
        .map_err(Error::ClientSetup)?
        .post(try_get_env_var("API_LOGIN")?)
        .header("X-Random-Challenge", random_challenge)
        .header("AppVersion", bundle_data.application_version_number.clone())
        .header("ContentBundle", bundle_data.version_number.clone())
        .header("DeviceId", generate_device_id(&email))
        .basic_auth(email, Some(password))
        .form(&[("grant_type", "client_credentials")])
        .send()
        .await
        .map_err(Error::Network)?
        .error_for_status()
        .map_err(|e| Error::BadStatus(e.status().unwrap_or(StatusCode::default())))?
        .json::<LoginResult>()
        .await
        .map_err(|e| Error::Decode(e.to_string()))
}

/// 通过xxxxxx api查询好友
///
/// # Errors
/// - 当环境变量配置无效时，返回[`Error::Env`]
/// - 当`reqwest`客户端初始化失败时，返回[`Error::ClientSetup`]
/// - 当请求发送失败时，返回[`Error::Network`]
/// - 当返回值不为2xx时，返回[`Error::BadStatus`]
/// - 当Json反序列化失败时，返回[`Error::Decode`]
pub async fn api_list_friend(
    bundle_data: &BundleData,
    email: String,
    user_id: String,
    token: String,
) -> Result<FriendListResult1> {
    reqwest::Client::builder()
        .user_agent("curl/7.88.1")
        .build()
        .map_err(Error::ClientSetup)?
        .get(try_get_env_var("API_LIST_FRIENDS")?)
        .header("X-Random-Challenge", generate_random_challenge())
        .header("Platform", "android")
        .header("AppVersion", bundle_data.application_version_number.clone())
        .header("ContentBundle", bundle_data.version_number.clone())
        .header("DeviceId", generate_device_id(&email))
        .header("i", user_id)
        .bearer_auth(token)
        .send()
        .await
        .map_err(Error::Network)?
        .error_for_status()
        .map_err(|e| Error::BadStatus(e.status().unwrap_or(StatusCode::default())))?
        .json::<FriendListResult>()
        .await
        .map_err(|e| Error::Decode(e.to_string()))
        .map(|it| it.value)
}

/// 通过xxxxxx api添加好友
///
/// # Errors
/// - 当环境变量配置无效时，返回[`Error::Env`]
/// - 当`reqwest`客户端初始化失败时，返回[`Error::ClientSetup`]
/// - 当请求发送失败时，返回[`Error::Network`]
/// - 当返回值不为2xx时，返回[`Error::BadStatus`]
/// - 当Json反序列化失败时，返回[`Error::Decode`]
pub async fn api_add_friend(
    bundle_data: &BundleData,
    email: String,
    user_id: String,
    token: String,
    friend_code: String,
) -> Result<FriendListResult1> {
    reqwest::Client::builder()
        .user_agent("curl/7.88.1")
        .build()
        .map_err(Error::ClientSetup)?
        .post(try_get_env_var("API_ADD_FRIENDS")?)
        .header("X-Random-Challenge", generate_random_challenge())
        .header("Platform", "android")
        .header("AppVersion", bundle_data.application_version_number.clone())
        .header("ContentBundle", bundle_data.version_number.clone())
        .header("DeviceId", generate_device_id(&email))
        .header("i", user_id)
        .bearer_auth(token)
        .form(&[("friend_code", friend_code)])
        .send()
        .await
        .map_err(Error::Network)?
        .error_for_status()
        .map_err(|e| Error::BadStatus(e.status().unwrap_or(StatusCode::default())))?
        .json::<FriendListResult>()
        .await
        .map_err(|e| Error::Decode(e.to_string()))
        .map(|it| it.value)
}

/// 通过xxxxxx api删除好友
///
/// # Errors
/// - 当环境变量配置无效时，返回[`Error::Env`]
/// - 当`reqwest`客户端初始化失败时，返回[`Error::ClientSetup`]
/// - 当请求发送失败时，返回[`Error::Network`]
/// - 当返回值不为2xx时，返回[`Error::BadStatus`]
/// - 当Json反序列化失败时，返回[`Error::Decode`]
pub async fn api_delete_friend(
    bundle_data: &BundleData,
    email: String,
    user_id: String,
    token: String,
    friend_id: String,
) -> Result<FriendListResult1> {
    reqwest::Client::builder()
        .user_agent("curl/7.88.1")
        .build()
        .map_err(Error::ClientSetup)?
        .post(try_get_env_var("API_DELETE_FRIENDS")?)
        .header("X-Random-Challenge", generate_random_challenge())
        .header("Platform", "android")
        .header("AppVersion", bundle_data.application_version_number.clone())
        .header("ContentBundle", bundle_data.version_number.clone())
        .header("DeviceId", generate_device_id(&email))
        .header("i", user_id)
        .bearer_auth(token)
        .form(&[("friend_id", friend_id)])
        .send()
        .await
        .map_err(Error::Network)?
        .error_for_status()
        .map_err(|e| Error::BadStatus(e.status().unwrap_or(StatusCode::default())))?
        .json::<FriendListResult>()
        .await
        .map_err(|e| Error::Decode(e.to_string()))
        .map(|it| it.value)
}

/// 通过xxxxxx api查询排行榜
///
/// # Errors
/// - 当环境变量配置无效时，返回[`Error::Env`]
/// - 当`reqwest`客户端初始化失败时，返回[`Error::ClientSetup`]
/// - 当请求发送失败时，返回[`Error::Network`]
/// - 当返回值不为2xx时，返回[`Error::BadStatus`]
/// - 当Json反序列化失败时，返回[`Error::Decode`]
#[allow(clippy::too_many_arguments)]
pub async fn api_get_rank_list(
    bundle_data: &BundleData,
    email: String,
    user_id: String,
    token: String,
    song_id: String,
    difficulty: String,
    start: String,
    limit: String,
) -> Result<Vec<SongScore>> {
    reqwest::Client::builder()
        .user_agent("curl/7.88.1")
        .build()
        .map_err(Error::ClientSetup)?
        .get(try_get_env_var("API_GET_RANK")?)
        .query(&[
            ("song_id", song_id),
            ("difficulty", difficulty),
            ("start", start),
            ("limit", limit),
        ])
        .header("X-Random-Challenge", generate_random_challenge())
        .header("Platform", "android")
        .header("AppVersion", bundle_data.application_version_number.clone())
        .header("ContentBundle", bundle_data.version_number.clone())
        .header("DeviceId", generate_device_id(&email))
        .header("i", user_id)
        .bearer_auth(token)
        .send()
        .await
        .map_err(Error::Network)?
        .error_for_status()
        .map_err(|e| Error::BadStatus(e.status().unwrap_or(StatusCode::default())))?
        .json::<SongScoreResult>()
        .await
        .map_err(|e| Error::Decode(e.to_string()))
        .map(|it| it.value)
}

fn generate_device_id(email: &str) -> String {
    // Warn: 这只是为了保证唯一性
    let mut result = String::with_capacity(16);
    for it in Sha256::digest(email).into_iter().take(8) {
        let _ = write!(&mut result, "{it:02x}");
    }
    result
}

/// 生成Random Challenge的占位符
///
/// 在不没有硬性检查的端点，就全部使用占位符替代
fn generate_random_challenge() -> String {
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into()
}

/// [`calc_friend_delta`]所使用的好友Delta量
pub enum FriendDelta {
    /// 好友相较`before`增加了一个
    Add(FriendInfo),
    /// 好友相较`before`减少了一个
    Remove(FriendInfo),
    /// 好友相较`before`没有变化
    Same,
}

/// 计算好友变化，返回好友信息
///
/// 当好友仅变化1个或不变化时，返回`Ok`
///
/// # Errors
/// 当好友变化量不为0或1时，返回`String`的`Err`\
/// （TODO：计划重构来改善错误处理）
///
/// # Panics
/// 所有可能的panic的代码来自于`set.iter().next().expect(...)`，而根据条件，这些set的长度在执行它之前均为1，故本函数理论上永远不会panic
pub fn calc_friend_delta(
    before: &[FriendInfo],
    after: &[FriendInfo],
) -> core::result::Result<FriendDelta, String> {
    let before: HashSet<_> = before.iter().collect();
    let after: HashSet<_> = after.iter().collect();
    let delta_add: HashSet<_> = after.difference(&before).collect();
    if delta_add.len() == 1 {
        return Ok(FriendDelta::Add(
            (**delta_add
                .iter()
                .next()
                .expect("first element of delta_add is None when len==1, this should not happen"))
            .clone(),
        ));
    }
    if !delta_add.is_empty() {
        return Err("bad add delta".into());
    }
    let delta_rem: HashSet<_> = before.difference(&after).collect();
    if delta_rem.len() == 1 {
        return Ok(FriendDelta::Remove(
            (**delta_rem
                .iter()
                .next()
                .expect("first element of delta_rem is None when len==1, this should not happen"))
            .clone(),
        ));
    }
    if !delta_rem.is_empty() {
        return Err("bad rem delta".into());
    }
    Ok(FriendDelta::Same)
}

/// 这个模块提供了便捷的xxxxxx api调用包装
pub mod auto {
    use std::time::Duration;

    use tokio::time::sleep;

    use crate::api;

    /// 一个包装，用于为一定量的[`StatusCode::TOO_MANY_REQUESTS`]错误提供弹性
    ///
    /// # Errors
    /// 当本函数因重试次数过多而失败时，返回[`api::Error::TooManyRetries`]\
    /// 否则，返回的错误由指定函数所可能引发的错误决定
    #[allow(clippy::cast_possible_truncation)]
    pub async fn xxxxxx_safe_call<'a, F, R, Fut>(
        max_retries: u64,
        exponential_backoff_base_millis: u64,
        exponential_backoff_multiplier: u64,
        worker_exponential_backoff_max_delay_millis: u64,
        f: F,
    ) -> Result<R, api::Error>
    where
        Fut: Future<Output = Result<R, api::Error>> + 'a + Send,
        R: Send + 'a,
        F: Fn() -> Fut,
    {
        //TODO add re-login
        let mut retries = 0;
        while retries <= max_retries {
            let result = f().await;
            match result {
                Ok(result) => return Ok(result),
                Err(api::Error::BadStatus(code)) if !code.is_success() => {
                    sleep(Duration::from_millis(
                        (exponential_backoff_base_millis
                            * exponential_backoff_multiplier.pow(retries as u32))
                        .min(worker_exponential_backoff_max_delay_millis),
                    ))
                    .await;
                }
                Err(e) => return Err(e),
            }
            retries += 1;
        }
        Err(api::Error::TooManyRetries)
    }
}
