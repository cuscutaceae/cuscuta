use std::sync::OnceLock;

use cuscuta_common::{
    data::{BundleData, Song},
    db::account::AccountRow,
};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub worker_max_jobs: u64,
    pub worker_max_retry_count: u64,
    pub worker_exponential_backoff_base_millis: u64,
    pub worker_exponential_backoff_multiplier: u64,
    pub worker_exponential_backoff_max_delay_millis: u64,
    pub redis_stream_refresh_ttl: i64,
    pub worker_account_lease_time_secs: u64,
    pub _worker_account_lease_time_refresh_gap_secs: u64,
    pub worker_job_max_work_time_secs: u64,
}

pub static BUNDLE_DATA: OnceLock<RwLock<Option<BundleData>>> = OnceLock::new();
pub static CONFIG: OnceLock<RwLock<Option<Config>>> = OnceLock::new();
pub static SONG_LIST: OnceLock<RwLock<Option<Vec<Song>>>> = OnceLock::new();

pub static ACCOUNT_ROW: OnceLock<RwLock<Option<AccountRow>>> = OnceLock::new();
