use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use reqwest::{RequestBuilder, Response, StatusCode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::api::LoggingRequestBeforeSend;
use crate::castable_enum;
use crate::{api::Error, data::AppVersionData};

type Result<T> = core::result::Result<T, Error>;

castable_enum!(
    /// 各种Api错误的转换
    #[allow(missing_docs)]
    #[derive(Debug, thiserror::Error, PartialEq, Eq)]
    #repr(i64)
    pub enum ApiErrorMap {
        #[error(
            "An error occurred completing purchases. Please try restarting your device or Xxxxxx and ensuring that you\"re logged in ,to."
        )]
        PurchaseFailed = 0,
        #[error("This item is currently unavailable to purchase.")]
        PurchaseUnavailable = 1,
        #[error("All songs are already downloaded!")]
        AllSongsAlreadyDownloaded = 2,
        #[error("You have been logged out by another device. Please restart Xxxxxx.")]
        LoggedOutByOtherDevice = 3,
        #[error("Could not connect to online server.")]
        CouldNotConnectToServer4 = 4,
        #[error("Incorrect app version.")]
        IncorrectAppVersion = 5,
        #[error("An unknown error has occurred.")]
        Unknown6 = 6,
        #[error("An unknown error has occurred.")]
        Unknown7 = 7,
        #[error("An unknown error has occurred.")]
        Unknown8 = 8,
        #[error("The Xxxxxx network is currently under maintenance.")]
        ServerInMaintenance = 9,
        #[error("An unknown error has occured.")]
        Unknown10 = 10,
        #[error("An unknown error has occured.")]
        Unknown11 = 11,
        #[error("Please update Xxxxxx to the latest version.")]
        UpdateVersionRequested12 = 12,
        #[error(
            "Registrations from this IP address are restricted.\nTry again later or contact support@yyyyyy.com."
        )]
        RegistrationsIPRestricted = 100,
        #[error("This username is already in use.")]
        DuplicatedUserName = 101,
        #[error("This email address is already in use.")]
        DuplicatedUserEmail = 102,
        #[error("An account has already been made from this device.")]
        RegistrationsDevicesRestricted = 103,
        #[error("Username or password incorrect.")]
        IncorrectUsernameOrPassword = 104,
        #[error(
            "You\"ve logged into over 2 devices in 24 hours. Please wait before using this new device."
        )]
        TooManyDevicesLogged = 105,
        #[error("This account is locked.")]
        UserLocked106 = 106,
        #[error("You do not have enough stamina.")]
        NoEnoughStamina = 107,
        #[error("An unknown error has occurred.")]
        Unknown108 = 108,
        #[error("An unknown error has occurred.")]
        Unknown109 = 109,
        #[error("An unknown error has occurred.")]
        Unknown110 = 110,
        #[error("An unknown error has occurred.")]
        Unknown111 = 111,
        #[error("World map not unlocked.")]
        WorldMapLocked = 112,
        #[error("This event map has ended and is no longer available.")]
        EventMapEnded = 113,
        #[error("An unknown error has occurred.")]
        Unknown114 = 114,
        #[error("An unknown error has occurred.")]
        Unknown115 = 115,
        #[error("An unknown error has occurred.")]
        Unknown116 = 116,
        #[error("An unknown error has occurred.")]
        Unknown117 = 117,
        #[error("An unknown error has occurred.")]
        Unknown118 = 118,
        #[error("An unknown error has occurred.")]
        Unknown119 = 119,
        #[error(
            "WARNING! You are using a modified version of Xxxxxx.\nContinued use will result in the banning of your account.\nThis ,is a final warning."
        )]
        ModifiedAppDetected = 120,
        #[error("This account is locked.")]
        UserLocked121 = 121,
        #[error(
            "A temporary hold has been placed on your account.\nPlease visit the official website to resolve the issue."
        )]
        UserTemporaryLocked = 122,
        #[error(
            "This feature has been restricted for your account.\nIf you are unsure why, please contact support@yyyyyy.com"
        )]
        FeatureRestricted = 150,
        #[error("This user does not exist.")]
        UserNotExist = 401,
        #[error("Could not connect to online server.")]
        CouldNotConnectToServer403 = 403,
        #[error("This item is currently unavailable to purchase.")]
        ItemUnavailableToPurchase501 = 501,
        #[error("This item is currently unavailable to purchase.")]
        ItemUnavailableToPurchase502 = 502,
        #[error("An unknown error has occured.")]
        Unknown503 = 503,
        #[error("Invalid Code")]
        InvalidCode = 504,
        #[error("This code has already been claimed.")]
        CodeAlreadyClaimed = 505,
        #[error("You already own this item.")]
        ItemAlreadyOwn = 506,
        #[error("You can\"t be friends with yourself ;-;")]
        AddSelfAsFriend = 604,
        #[error("Your friends list is full.")]
        FriendListIsFull = 601,
        #[error("This user is already your friend.")]
        UserIsAlreadyFriend = 602,
        #[error(
            "There was a problem receiving the server response. Please check your progress after re-entering World Mode."
        )]
        WorldServerProblem = 801,
        #[error("This score could not be submitted online. Please restart or update Xxxxxx.")]
        ScoreCouldNotBeSubmitted = 802,
        #[error(
            "There was a problem submitting this score online. WARNING!Stamina has already been consumed. Exiting will lose World ,Mode progress."
        )]
        ScoreCouldNotBeSubmittedWithStaminaLost = 803,
        #[error("Password reset expired. Please request a new reset link.")]
        PasswordResetExpired = 804,
        #[error("")]
        Unknown805 = 805,
        #[error("Max downloads exceeded. Please wait 24 hours and try again.")]
        MaxDownloadExceeded = 903,
        #[error("Please wait 24 hours before using this feature again.")]
        FeatureCoolingDown = 905,
        #[error(
            "Game data is out of sync due to another device. Please check your progress after re-entering World Mode."
        )]
        WorldDataOutOfSync = 9701,
        #[error("An error occured downloading the song.Please try again.")]
        DownloadSongFailed = 9801,
        #[error("There was a problem saving the song.Please check storage.")]
        SaveSongFailed = 9802,
        #[error("No data found to sync.")]
        NoDataFoundToSync = 9905,
        #[error(
            "Sync failed due to conflicting data from another device. Please perform sync from Main Menu > Network."
        )]
        SyncFailedDueToConflictingData = 9906,
        #[error("A problem occured updating data...")]
        UpdatingDataFailed = 9907,
        #[error("There is a new version of Xxxxxx available.Please update.")]
        UpdateVersionRequested9908 = 9908,
        #[error("cuscuta does not know this error")]
        Unknown = -1,
    }
);

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

/// Xxxxxx 的 API URL
#[derive(Debug, Clone)]
pub struct XxxxxxUrl {
    /// 登录 Url
    pub login: String,

    /// 好友列表 Url
    pub list_friend: String,

    /// 添加好友 Url
    pub add_friend: String,

    /// 删除好友 Url
    pub delete_friend: String,

    /// 排行榜 Url
    pub get_rank: String,

    /// 通知 Url
    pub get_notification: String,

    /// 聚合调用 Url
    pub compose: String,
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
    url: &XxxxxxUrl,
    bundle_data: &AppVersionData,
    email: &str,
    password: &str,
    random_challenge: &str,
) -> Result<LoginResult> {
    reqwest::Client::builder()
        .build()
        .map_err(Error::ClientSetup)?
        .post(&url.login)
        .attach_random_challenge(random_challenge)
        .attach_version(bundle_data)
        .attach_device_id(email)
        .basic_auth(email, Some(password))
        .form(&[("grant_type", "client_credentials")])
        .send_after_log()
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
    url: &XxxxxxUrl,
    bundle_data: &AppVersionData,
    email: &str,
    user_id: &str,
    token: &str,
    random_challenge: &str,
) -> Result<FriendListResult1> {
    reqwest::Client::builder()
        .build()
        .map_err(Error::ClientSetup)?
        .get(&url.list_friend)
        .attach_random_challenge(random_challenge)
        .attach_platform()
        .attach_version(bundle_data)
        .attach_device_id(email)
        .attach_user_id(user_id)
        .bearer_auth(token)
        .send_after_log()
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
    url: &XxxxxxUrl,
    bundle_data: &AppVersionData,
    email: &str,
    user_id: &str,
    token: &str,
    friend_code: &str,
    random_challenge: &str,
) -> Result<FriendListResult1> {
    reqwest::Client::builder()
        .build()
        .map_err(Error::ClientSetup)?
        .post(&url.add_friend)
        .attach_random_challenge(random_challenge)
        .attach_platform()
        .attach_version(bundle_data)
        .attach_device_id(email)
        .attach_user_id(user_id)
        .bearer_auth(token)
        .form(&[("friend_code", friend_code)])
        .send_after_log()
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
    url: &XxxxxxUrl,
    bundle_data: &AppVersionData,
    email: &str,
    user_id: &str,
    token: &str,
    friend_id: &str,
    random_challenge: &str,
) -> Result<FriendListResult1> {
    reqwest::Client::builder()
        .build()
        .map_err(Error::ClientSetup)?
        .post(&url.delete_friend)
        .attach_random_challenge(random_challenge)
        .attach_platform()
        .attach_version(bundle_data)
        .attach_device_id(email)
        .attach_user_id(user_id)
        .bearer_auth(token)
        .form(&[("friend_id", friend_id)])
        .send_after_log()
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
    url: &XxxxxxUrl,
    bundle_data: &AppVersionData,
    email: &str,
    user_id: &str,
    token: &str,
    song_id: &str,
    difficulty: &str,
    start: &str,
    limit: &str,
    random_challenge: &str,
) -> Result<Vec<SongScore>> {
    reqwest::Client::builder()
        .build()
        .map_err(Error::ClientSetup)?
        .get(&url.get_rank)
        .query(&[
            ("song_id", song_id),
            ("difficulty", difficulty),
            ("start", start),
            ("limit", limit),
        ])
        .attach_random_challenge(random_challenge)
        .attach_platform()
        .attach_version(bundle_data)
        .attach_device_id(email)
        .attach_user_id(user_id)
        .bearer_auth(token)
        .send_after_log()
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

/// 通过xxxxxx api查询通知
///
/// 本操作的返回将被丢弃
///
/// # Errors
/// - 当环境变量配置无效时，返回[`Error::Env`]
/// - 当`reqwest`客户端初始化失败时，返回[`Error::ClientSetup`]
/// - 当请求发送失败时，返回[`Error::Network`]
/// - 当返回值不为2xx时，返回[`Error::BadStatus`]
pub async fn api_get_notification(
    url: &XxxxxxUrl,
    bundle_data: &AppVersionData,
    email: &str,
    user_id: &str,
    token: &str,
    random_challenge: &str,
) -> Result<()> {
    reqwest::Client::builder()
        .build()
        .map_err(Error::ClientSetup)?
        .get(&url.get_notification)
        .attach_random_challenge(random_challenge)
        .attach_platform()
        .attach_version(bundle_data)
        .attach_device_id(email)
        .attach_user_id(user_id)
        .bearer_auth(token)
        .send_after_log()
        .await
        .map_err(Error::Network)?
        .error_for_status_with_response()
        .await
        .map(|_| ())
}

/// 通过xxxxxx api进行聚合查询
///
/// 本操作的返回将被丢弃
///
/// # Errors
/// - 当环境变量配置无效时，返回[`Error::Env`]
/// - 当`reqwest`客户端初始化失败时，返回[`Error::ClientSetup`]
/// - 当请求发送失败时，返回[`Error::Network`]
/// - 当返回值不为2xx时，返回[`Error::BadStatus`]
pub async fn api_compose_aggregate(
    url: &XxxxxxUrl,
    bundle_data: &AppVersionData,
    email: &str,
    user_id: &str,
    token: &str,
    random_challenge: &str,
) -> Result<()> {
    tracing::debug!("XXXXXX: {token}");
    reqwest::Client::builder()
        .build()
        .map_err(Error::ClientSetup)?
        .get(&url.compose)
        .attach_random_challenge(random_challenge)
        .attach_platform()
        .attach_version(bundle_data)
        .attach_device_id(email)
        .attach_user_id(user_id)
        .bearer_auth(token)
        .send_after_log()
        .await
        .map_err(Error::Network)?
        .error_for_status_with_response()
        .await
        .map(|_| ())
}

fn generate_device_id(email: &str) -> String {
    // Warn: 这只是为了保证唯一性
    let mut result = String::with_capacity(16);
    for it in Sha256::digest(email).into_iter().take(8) {
        let _ = write!(&mut result, "{it:02x}");
    }
    result
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
/// 当好友变化量不为0或1时，返回`String`的`Err`
///
/// # Panics
/// 所有可能的panic的代码来自于`set.iter().next().expect(...)`，而根据条件，这些set的长度在执行它之前均为1，故本函数理论上永远不会panic
pub fn calc_friend_delta(
    before: &[FriendInfo],
    after: &[FriendInfo],
) -> core::result::Result<FriendDelta, String> {
    let before_id: HashMap<_, _> = before.iter().map(|it| (it.user_id, it)).collect();
    let after_id: HashMap<_, _> = after.iter().map(|it| (it.user_id, it)).collect();
    let delta_add: Vec<_> = after_id
        .keys()
        .collect::<HashSet<_>>()
        .difference(&before_id.keys().collect::<HashSet<_>>())
        .map(|it| after_id.get(*it).expect("should exist"))
        .collect();
    if delta_add.len() == 1 {
        return Ok(FriendDelta::Add(
            (**delta_add
                .first()
                .expect("first element of delta_add is None when len==1, this should not happen"))
            .clone(),
        ));
    }
    if !delta_add.is_empty() {
        return Err("bad add delta".into());
    }
    let delta_rem: Vec<_> = before_id
        .keys()
        .collect::<HashSet<_>>()
        .difference(&after_id.keys().collect::<HashSet<_>>())
        .map(|it| after_id.get(*it).expect("should exist"))
        .collect();
    if delta_rem.len() == 1 {
        return Ok(FriendDelta::Remove(
            (**delta_rem
                .first()
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
        fn parse(api_error: &ApiError, status_code: StatusCode) -> Error {
            let error_code = api_error.error_code;
            let error_map = ApiErrorMap::from(error_code);
            Error::BadStatus {
                status_code,
                extra_error_code: Some(error_code),
                message: if error_map == ApiErrorMap::Unknown {
                    format!("failed to map error description: code {error_code}")
                } else {
                    error_map.to_string()
                },
            }
        }
        match self.error_for_status_ref() {
            Ok(_) => Ok(self),
            Err(e) => {
                let status_code = self.status();
                let text = self.text().await.map_err(|it| Error::BadStatus {
                    status_code,
                    message: format!("[FAILED TO GET BODY: {it} {e}]"),
                    extra_error_code: None,
                })?;
                let api_error = serde_json::from_str::<ApiError>(text.as_str()).map_err(|it| {
                    Error::BadStatus {
                        status_code,
                        message: format!("[FAILED TO PARSE JSON: {it} {e}]"),
                        extra_error_code: None,
                    }
                })?;
                Err(parse(&api_error, status_code))
            }
        }
    }
}

trait RequestBuilderHelper {
    fn attach_platform(self) -> Self;

    fn attach_version(self, bundle_data: &AppVersionData) -> Self;

    fn attach_device_id(self, email: &str) -> Self;

    fn attach_user_id(self, user_id: &str) -> Self;

    fn attach_random_challenge(self, random_challenge: &str) -> Self;
}

impl RequestBuilderHelper for RequestBuilder {
    fn attach_platform(self) -> Self {
        self.header("Platform", "android")
    }

    fn attach_version(self, bundle_data: &AppVersionData) -> Self {
        let version = &bundle_data.version;
        let version = if version.ends_with('c') {
            version.clone()
        } else {
            format!("{version}c")
        };
        self.header("AppVersion", version)
    }

    fn attach_device_id(self, email: &str) -> Self {
        self.header("DeviceId", generate_device_id(email))
    }

    fn attach_user_id(self, user_id: &str) -> Self {
        self.header("i", user_id)
    }

    fn attach_random_challenge(self, random_challenge: &str) -> Self {
        self.header("X-Random-Challenge", random_challenge)
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
