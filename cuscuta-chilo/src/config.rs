use std::sync::OnceLock;

use cuscuta_common::config::{
    IntoPostgresqlUrlAddress, IntoRedisUrlAddress, IntoResourcesUpdatePeriodAndRetries,
    IntoSongUrlAddress, default_account_lease_time_refresh_gap_secs,
    default_account_lease_time_secs, default_empty_friends_delay_time_secs,
    default_exponential_backoff_base_millis, default_exponential_backoff_max_delay_millis,
    default_exponential_backoff_multiplier, default_job_max_work_time_secs, default_max_jobs,
    default_max_retries, default_redis_stream_refresh_ttl, default_resource_update_period,
    default_resource_update_retries, default_resources_app_version_use_online,
    default_use_online_key, default_use_online_prefix,
};
use serde::Deserialize;

static ENV: OnceLock<Environment> = OnceLock::new();

#[derive(Debug, Deserialize)]
pub struct Environment {
    #[serde(default = "default_resource_update_period")]
    pub resource_update_period: u64,
    #[serde(default = "default_resource_update_retries")]
    pub resource_update_retries: u64,
    #[serde(default = "default_use_online_key")]
    pub use_online_key: bool,
    pub scirpophaga_url: Option<String>,
    pub bin_c2: Option<String>,
}

pub fn init_env() -> Result<&'static Environment, envy::Error> {
    cuscuta_common::config::init_env(&ENV)
}

pub fn fetch_env() -> &'static Environment {
    cuscuta_common::config::fetch_env(&ENV)
}
