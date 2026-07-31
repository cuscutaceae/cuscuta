use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::sync::LazyLock;

use reqwest::Response;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    api::{Error, try_get_env_var},
    data::BundleData,
};

type Result<T> = core::result::Result<T, Error>;

static ERROR_MAP: LazyLock<HashMap<i64, &str>> = LazyLock::new(|| {
    HashMap::from([
        (
            0,
            "An error occurred completing purchases. Please try restarting your device or Arcaea and ensuring that you\"re logged in ,to.",
        ),
        (1, "This item is currently unavailable to purchase."),
        (2, "All songs are already downloaded!"),
        (
            3,
            "You have been logged out by another device. Please restart Arcaea.",
        ),
        (4, "Could not connect to online server."),
        (5, "Incorrect app version."),
        (6, "An unknown error has occurred."),
        (7, "An unknown error has occurred."),
        (8, "An unknown error has occurred."),
        (9, "The Arcaea network is currently under maintenance."),
        (10, "An unknown error has occured."),
        (11, "An unknown error has occured."),
        (12, "Please update Arcaea to the latest version."),
        (
            100,
            "Registrations from this IP address are restricted.\nTry again later or contact support@lowiro.com.",
        ),
        (101, "This username is already in use."),
        (102, "This email address is already in use."),
        (103, "An account has already been made from this device."),
        (104, "Username or password incorrect."),
        (
            105,
            "You\"ve logged into over 2 devices in 24 hours. Please wait before using this new device.",
        ),
        (106, "This account is locked."),
        (107, "You do not have enough stamina."),
        (108, "An unknown error has occurred."),
        (109, "An unknown error has occurred."),
        (110, "An unknown error has occurred."),
        (111, "An unknown error has occurred."),
        (112, "World map not unlocked."),
        (113, "This event map has ended and is no longer available."),
        (114, "An unknown error has occurred."),
        (115, "An unknown error has occurred."),
        (116, "An unknown error has occurred."),
        (117, "An unknown error has occurred."),
        (118, "An unknown error has occurred."),
        (119, "An unknown error has occurred."),
        (
            120,
            "WARNING! You are using a modified version of Arcaea.\nContinued use will result in the banning of your account.\nThis ,is a final warning.",
        ),
        (121, "This account is locked."),
        (
            122,
            "A temporary hold has been placed on your account.\nPlease visit the official website to resolve the issue.",
        ),
        (
            150,
            "This feature has been restricted for your account.\nIf you are unsure why, please contact support@lowiro.com",
        ),
        (401, "This user does not exist."),
        (403, "Could not connect to online server."),
        (501, "This item is currently unavailable to purchase."),
        (502, "This item is currently unavailable to purchase."),
        (503, "An unknown error has occured."),
        (504, "Invalid Code"),
        (505, "This code has already been claimed."),
        (506, "You already own this item."),
        (604, "You can\"t be friends with yourself ;-;"),
        (601, "Your friends list is full."),
        (602, "This user is already your friend."),
        (
            801,
            "There was a problem receiving the server response. Please check your progress after re-entering World Mode.",
        ),
        (
            802,
            "This score could not be submitted online. Please restart or update Arcaea.",
        ),
        (
            803,
            "There was a problem submitting this score online. WARNING!Stamina has already been consumed. Exiting will lose World ,Mode progress.",
        ),
        (
            804,
            "Password reset expired. Please request a new reset link.",
        ),
        (805, ""),
        (
            903,
            "Max downloads exceeded. Please wait 24 hours and try again.",
        ),
        (905, "Please wait 24 hours before using this feature again."),
        (
            9701,
            "Game data is out of sync due to another device. Please check your progress after re-entering World Mode.",
        ),
        (
            9801,
            "An error occured downloading the song.Please try again.",
        ),
        (
            9802,
            "There was a problem saving the song.Please check storage.",
        ),
        (9905, "No data found to sync."),
        (
            9906,
            "Sync failed due to conflicting data from another device. Please perform sync from Main Menu > Network.",
        ),
        (9907, "A problem occured updating data..."),
        (
            9908,
            "There is a new version of Arcaea available.Please update.",
        ),
    ])
});

/// xxxxxx api的通用错误
#[derive(Debug, Deserialize, Clone)]
pub struct ApiError {
    /// 错误码
    pub error_code: i64,
}

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
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct FriendInfo {
    /// 好友游戏名
    pub name: String,

    /// 好友用户id
    pub user_id: i64,

    /// 好友的评级（实际显示评级为`rating/10.0`）
    pub rating: i64,

    /// 好友设置的搭档
    pub character: i64,

    /// 好友搭档状态1
    pub is_char_uncapped: bool,

    /// 好友搭档状态2
    pub is_char_uncapped_override: bool,
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

impl From<ApiError> for Error {
    fn from(value: ApiError) -> Self {
        let error_code = value.error_code;
        Self::ApiError {
            error_code,
            message: ERROR_MAP.get(&error_code).copied().map_or_else(
                || format!("failed to map error description: code {error_code}"),
                &str::to_string,
            ),
        }
    }
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
    email: &str,
    password: &str,
    random_challenge: &str,
) -> Result<LoginResult> {
    reqwest::Client::builder()
        .user_agent("curl/7.88.1")
        .build()
        .map_err(Error::ClientSetup)?
        .post(try_get_env_var("API_LOGIN")?)
        .header("X-Random-Challenge", random_challenge)
        .header("AppVersion", bundle_data.application_version_number.clone())
        .header("ContentBundle", bundle_data.version_number.clone())
        .header("DeviceId", generate_device_id(email))
        .basic_auth(email, Some(password))
        .form(&[("grant_type", "client_credentials")])
        .send()
        .await
        .map_err(Error::Network)?
        .error_for_status_with_response()
        .await?
        .json::<LoginResult>()
        .await
        .map_err(|e| Error::Decode {
            message: e.to_string(),
        })
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
    email: &str,
    user_id: &str,
    token: &str,
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
        .header("DeviceId", generate_device_id(email))
        .header("i", user_id)
        .bearer_auth(token)
        .send()
        .await
        .map_err(Error::Network)?
        .error_for_status_with_response()
        .await?
        .json::<FriendListResult>()
        .await
        .map_err(|e| Error::Decode {
            message: e.to_string(),
        })
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
    email: &str,
    user_id: &str,
    token: &str,
    friend_code: &str,
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
        .header("DeviceId", generate_device_id(email))
        .header("i", user_id)
        .bearer_auth(token)
        .form(&[("friend_code", friend_code)])
        .send()
        .await
        .map_err(Error::Network)?
        .error_for_status_with_response()
        .await?
        .json::<FriendListResult>()
        .await
        .map_err(|e| Error::Decode {
            message: e.to_string(),
        })
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
    email: &str,
    user_id: &str,
    token: &str,
    friend_id: &str,
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
        .header("DeviceId", generate_device_id(email))
        .header("i", user_id)
        .bearer_auth(token)
        .form(&[("friend_id", friend_id)])
        .send()
        .await
        .map_err(Error::Network)?
        .error_for_status_with_response()
        .await?
        .json::<FriendListResult>()
        .await
        .map_err(|e| Error::Decode {
            message: e.to_string(),
        })
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
    email: &str,
    user_id: &str,
    token: &str,
    song_id: &str,
    difficulty: &str,
    start: &str,
    limit: &str,
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
        .header("DeviceId", generate_device_id(email))
        .header("i", user_id)
        .bearer_auth(token)
        .send()
        .await
        .map_err(Error::Network)?
        .error_for_status_with_response()
        .await?
        .json::<SongScoreResult>()
        .await
        .map_err(|e| Error::Decode {
            message: e.to_string(),
        })
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

trait ErrorForStatusWithResponseXxxxxx
where
    Self: Sized,
{
    fn error_for_status_with_response(
        self,
    ) -> impl Future<Output = std::result::Result<Self, Error>>;
}

impl ErrorForStatusWithResponseXxxxxx for Response {
    async fn error_for_status_with_response(self) -> std::result::Result<Self, Error> {
        match self.error_for_status_ref() {
            Ok(_) => Ok(self),
            Err(e) => {
                let status_code = self.status();
                let text = self.text().await.map_err(|it| Error::BadStatus {
                    status_code,
                    message: format!("[FAILED TO GET BODY: {it} {e}]"),
                })?;
                let err = serde_json::from_str::<ApiError>(text.as_str()).map_err(|it| {
                    Error::BadStatus {
                        status_code,
                        message: format!("[FAILED TO PARSE JSON: {it} {e}]"),
                    }
                })?;
                Err(err.into())
            }
        }
    }
}

/// 这个模块提供了便捷的xxxxxx api调用包装
pub mod auto {
    use std::time::Duration;

    use reqwest::StatusCode;
    use tokio::time::{Sleep, sleep};

    use crate::api;

    /// 一个包装，用于为一定量的[`StatusCode`]错误和网络错误提供弹性
    /// 使用简化的错误判断，任何错误均重试
    ///
    /// # Errors
    /// 当本函数因重试次数过多而失败时，返回[`api::Error::TooManyRetries`]\
    /// 否则，返回的错误由指定函数所可能引发的错误决定
    #[allow(clippy::cast_possible_truncation)]
    pub async fn xxxxxx_safe_call<'a, F, R, Fut>(
        max_retries: u64,
        exponential_backoff_base_millis: u64,
        exponential_backoff_multiplier: u64,
        exponential_backoff_max_delay_millis: u64,
        f: F,
    ) -> Result<R, api::Error>
    where
        Fut: Future<Output = Result<R, api::Error>> + 'a + Send,
        R: Send + 'a,
        F: Fn() -> Fut,
    {
        xxxxxx_safe_call_ex(
            max_retries,
            exponential_backoff_base_millis,
            exponential_backoff_multiplier,
            exponential_backoff_max_delay_millis,
            |_| false,
            f,
        )
        .await
    }

    /// 一个包装，用于为一定量的[`StatusCode`]错误和网络错误提供弹性
    ///
    /// # Errors
    /// 当本函数因重试次数过多而失败时，返回[`api::Error::TooManyRetries`]\
    /// 否则，返回的错误由指定函数所可能引发的错误决定
    ///
    /// # Panics
    /// 本函数不会因为除不合理传入`max_retries`或`exponential_backoff_multiplier`以外的情况下panic，
    /// 本函数的panic来自最后返回的`expect`，当`latest_error`为`None`时panic，
    /// 但由于`retries < max_retries.max(1)`，故循环体至少会被执行一次，
    /// 故在最后一行可达的情况下，`latest_error`不可能为`None`，
    /// 故一般使用情况下，本函数不会panic ~（除非调用者故意找茬）~
    #[allow(clippy::cast_possible_truncation)]
    pub async fn xxxxxx_safe_call_ex<'a, F, R, T, Fut>(
        max_retries: u64,
        exponential_backoff_base_millis: u64,
        exponential_backoff_multiplier: u64,
        exponential_backoff_max_delay_millis: u64,
        fail_cond: T,
        f: F,
    ) -> Result<R, api::Error>
    where
        Fut: Future<Output = Result<R, api::Error>> + 'a + Send,
        R: Send + 'a,
        F: Fn() -> Fut,
        T: Fn(StatusCode) -> bool,
    {
        //TODO add re-login
        fn wait(
            exponential_backoff_base_millis: u64,
            exponential_backoff_multiplier: u64,
            exponential_backoff_max_delay_millis: u64,
            retries: u64,
        ) -> Sleep {
            sleep(Duration::from_millis(
                (exponential_backoff_base_millis
                    * exponential_backoff_multiplier.saturating_pow(retries as u32))
                .min(exponential_backoff_max_delay_millis),
            ))
        }
        let mut retries = 0;
        let mut latest_error = None;
        while retries < max_retries.max(1) {
            let result = f().await;
            match result {
                Ok(result) => return Ok(result),
                Err(e) => {
                    match &e {
                        api::Error::Network(_) => {
                            wait(
                                exponential_backoff_base_millis,
                                exponential_backoff_multiplier,
                                exponential_backoff_max_delay_millis,
                                retries,
                            )
                            .await;
                        }
                        api::Error::BadStatus { status_code, .. } if !fail_cond(*status_code) => {
                            wait(
                                exponential_backoff_base_millis,
                                exponential_backoff_multiplier,
                                exponential_backoff_max_delay_millis,
                                retries,
                            )
                            .await;
                        }
                        _ => return Err(e),
                    }
                    latest_error = Some(e);
                }
            }
            retries += 1;
        }
        Err(latest_error.expect("this should not happen"))
    }
}
