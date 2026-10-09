use std::sync::OnceLock;

use serde::Deserialize;

/// 默认的Redis stream刷新TTL
#[must_use]
pub const fn default_redis_stream_refresh_ttl() -> i64 {
    300
}

/// 默认的资源更新间隔
#[must_use]
pub const fn default_resource_update_period() -> u64 {
    60
}

/// 默认的资源更新重试次数
#[must_use]
pub const fn default_resource_update_retries() -> u64 {
    5
}

/// 默认的 stat 激活开关
#[must_use]
pub const fn default_stat_enable() -> bool {
    true
}

/// 默认的在线获取 App 版本开关
#[must_use]
pub const fn default_resources_app_version_use_online() -> bool {
    true
}

/// 默认的在线拉取密钥开关
#[must_use]
pub const fn default_use_online_key() -> bool {
    true
}

/// 默认的在线获取 API 路径前缀开关
#[must_use]
pub const fn default_use_online_prefix() -> bool {
    true
}

/// 默认的每个 worker 最大并发任务数
#[must_use]
pub const fn default_max_jobs() -> u64 {
    8
}

/// 默认的 API 调用最大重试次数
#[must_use]
pub const fn default_max_retries() -> u64 {
    30
}

/// 默认的指数退避初始等待时间（毫秒）
#[must_use]
pub const fn default_exponential_backoff_base_millis() -> u64 {
    10
}

/// 默认的指数退避乘数
#[must_use]
pub const fn default_exponential_backoff_multiplier() -> u64 {
    2
}

/// 默认的指数退避最大等待时间（毫秒）
#[must_use]
pub const fn default_exponential_backoff_max_delay_millis() -> u64 {
    500
}

/// 默认的账号租约时长（秒）
#[must_use]
pub const fn default_account_lease_time_secs() -> u64 {
    120
}

/// 默认的租约续期间隔（秒）
#[must_use]
pub const fn default_account_lease_time_refresh_gap_secs() -> u64 {
    30
}

/// 默认的任务最长运行时间（秒）
#[must_use]
pub const fn default_job_max_work_time_secs() -> u64 {
    1200
}

/// 默认的风控应对延迟（秒）
#[must_use]
pub const fn default_empty_friends_delay_time_secs() -> u64 {
    10
}

/// 提取更新间隔和重试次数的trait
pub trait IntoResourcesUpdatePeriodAndRetries {
    /// 资源更新间隔
    fn resource_update_period(&self) -> u64;

    /// 资源更新重试次数
    fn resource_update_retries(&self) -> u64;
}

/// 提取 Redis URL 的 trait
pub trait IntoRedisUrlAddress {
    /// Redis URL
    fn redis_address(&self) -> &str;
}

/// 提取 Postgresql URL 的 trait
pub trait IntoPostgresqlUrlAddress {
    /// Postgresql URL
    fn postgresql_address(&self) -> &str;
}

/// 提取曲目信息 URL 的 trait
pub trait IntoSongUrlAddress {
    /// song list URL
    fn song_list_url_address(&self) -> &str;
}

/// 初始化环境变量
///
/// # Errors
/// 如果转换失败，返回[`envy::Error`]
pub fn init_env<T>(global_env: &'static OnceLock<T>) -> Result<&'static T, envy::Error>
where
    T: for<'a> Deserialize<'a>,
{
    let env = envy::from_env::<T>()?;
    global_env.get_or_init(|| env);
    Ok(fetch_env(global_env))
}

/// 获取环境变量
///
/// # Panics
/// 如果在调用此函数此前没有初始化`global_env`，则 panic
pub fn fetch_env<T>(global_env: &'static OnceLock<T>) -> &'static T {
    global_env
        .get()
        .expect("should use after a success call of init_env at least")
}
