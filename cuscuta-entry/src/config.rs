use std::sync::OnceLock;

use cuscuta_common::config::{
    IntoPostgresqlUrlAddress, IntoRedisUrlAddress, IntoResourcesUpdatePeriodAndRetries,
    IntoSongUrlAddress, default_redis_stream_refresh_ttl, default_resource_update_period,
    default_resource_update_retries, default_stat_enable,
};
use serde::Deserialize;

static ENV: OnceLock<Environment> = OnceLock::new();

#[derive(Debug, Deserialize, Clone)]
pub struct Environment {
    #[serde(default = "default_redis_stream_refresh_ttl")]
    pub redis_stream_refresh_ttl: i64,
    #[serde(default = "default_stat_enable")]
    pub stat_enable: bool,
    #[serde(default = "default_resource_update_period")]
    pub resource_update_period: u64,
    #[serde(default = "default_resource_update_retries")]
    pub resource_update_retries: u64,
    pub redis_addr: String,
    pub accounts_sql_addr: String,
    pub resources_song_url: String,
    pub cors_allow_origins: Option<String>,
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
