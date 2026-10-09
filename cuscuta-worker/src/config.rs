use std::sync::OnceLock;

use cuscuta_common::config::{
    IntoPostgresqlUrlAddress, IntoRedisUrlAddress, IntoResourcesUpdatePeriodAndRetries,
    IntoSongUrlAddress, default_account_lease_time_refresh_gap_secs,
    default_account_lease_time_secs, default_empty_friends_delay_time_secs,
    default_exponential_backoff_base_millis, default_exponential_backoff_max_delay_millis,
    default_exponential_backoff_multiplier, default_job_max_work_time_secs, default_max_jobs,
    default_max_retries, default_redis_stream_refresh_ttl, default_resource_update_period,
    default_resource_update_retries, default_resources_app_version_use_online,
    default_use_online_prefix,
};
use serde::Deserialize;

static ENV: OnceLock<Environment> = OnceLock::new();

#[derive(Debug, Deserialize, Clone)]
pub struct Environment {
    pub redis_addr: String,
    pub accounts_sql_addr: String,
    #[serde(default = "default_resource_update_period")]
    pub resource_update_period: u64,
    #[serde(default = "default_resource_update_retries")]
    pub resource_update_retries: u64,
    #[serde(default = "default_resources_app_version_use_online")]
    pub resources_app_version_use_online: bool,
    #[serde(default = "default_use_online_prefix")]
    pub use_online_prefix: bool,
    pub scirpophaga_url: Option<String>,
    pub resources_app_version_default: Option<String>,
    pub resources_song_url: String,
    pub resources_app_version_url: Option<String>,
    pub api_prefix_auth: Option<String>,
    pub api_prefix_common: Option<String>,
    pub api_chilo: String,
    pub api_endpoint_login: String,
    pub api_endpoint_list_friends: String,
    pub api_endpoint_add_friends: String,
    pub api_endpoint_delete_friends: String,
    pub api_endpoint_get_rank: String,
    pub api_endpoint_notification: String,
    pub api_endpoint_compose_aggregate: String,
    #[serde(default = "default_max_jobs")]
    pub worker_max_jobs: u64,
    #[serde(default = "default_max_retries")]
    pub worker_max_retries: u64,
    #[serde(default = "default_exponential_backoff_base_millis")]
    pub worker_exponential_backoff_base_millis: u64,
    #[serde(default = "default_exponential_backoff_multiplier")]
    pub worker_exponential_backoff_multiplier: u64,
    #[serde(default = "default_exponential_backoff_max_delay_millis")]
    pub worker_exponential_backoff_max_delay_millis: u64,
    #[serde(default = "default_account_lease_time_secs")]
    pub worker_account_lease_time_secs: u64,
    #[serde(default = "default_account_lease_time_refresh_gap_secs")]
    pub worker_account_lease_time_refresh_gap_secs: u64,
    #[serde(default = "default_job_max_work_time_secs")]
    pub worker_job_max_work_time_secs: u64,
    #[serde(default = "default_empty_friends_delay_time_secs")]
    pub worker_empty_friends_delay_time_secs: u64,
    #[serde(default = "default_redis_stream_refresh_ttl")]
    pub redis_stream_refresh_ttl: i64,
}

impl IntoResourcesUpdatePeriodAndRetries for Environment {
    fn resource_update_period(&self) -> u64 {
        self.resource_update_period
    }

    fn resource_update_retries(&self) -> u64 {
        self.resource_update_retries
    }
}

impl IntoRedisUrlAddress for Environment {
    fn redis_address(&self) -> &str {
        &self.redis_addr
    }
}

impl IntoPostgresqlUrlAddress for Environment {
    fn postgresql_address(&self) -> &str {
        &self.accounts_sql_addr
    }
}

impl IntoSongUrlAddress for Environment {
    fn song_list_url_address(&self) -> &str {
        &self.resources_song_url
    }
}

pub fn init_env() -> Result<&'static Environment, envy::Error> {
    cuscuta_common::config::init_env(&ENV)
}

pub fn fetch_env() -> &'static Environment {
    cuscuta_common::config::fetch_env(&ENV)
}

#[cfg(test)]
pub fn load_env(env: Environment) {
    ENV.get_or_init(|| env);
}

#[cfg(test)]
pub mod mock {
    use cuscuta_test::mock::SimpleMockable;

    use crate::config::Environment;

    impl SimpleMockable for Environment {
        fn mock() -> Self {
            Self {
                worker_max_jobs: 8,
                worker_max_retries: 5,
                worker_exponential_backoff_base_millis: 100,
                worker_exponential_backoff_multiplier: 2,
                worker_exponential_backoff_max_delay_millis: 500,
                redis_stream_refresh_ttl: 600,
                worker_account_lease_time_secs: 60,
                worker_account_lease_time_refresh_gap_secs: 5,
                worker_job_max_work_time_secs: 600,
                worker_empty_friends_delay_time_secs: 10,
                redis_addr: String::new(),
                accounts_sql_addr: String::new(),
                resource_update_period: Default::default(),
                resource_update_retries: Default::default(),
                resources_app_version_use_online: Default::default(),
                use_online_prefix: Default::default(),
                scirpophaga_url: String::new().into(),
                resources_app_version_default: String::new().into(),
                resources_song_url: String::new(),
                resources_app_version_url: String::new().into(),
                api_prefix_auth: String::new().into(),
                api_prefix_common: String::new().into(),
                api_chilo: String::new(),
                api_endpoint_login: String::new(),
                api_endpoint_list_friends: String::new(),
                api_endpoint_add_friends: String::new(),
                api_endpoint_delete_friends: String::new(),
                api_endpoint_get_rank: String::new(),
                api_endpoint_notification: String::new(),
                api_endpoint_compose_aggregate: String::new(),
            }
        }
    }
}
