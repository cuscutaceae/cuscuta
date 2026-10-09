use std::sync::OnceLock;

use cuscuta_common::config::{
    default_resource_update_period, default_resource_update_retries, default_use_online_key,
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
