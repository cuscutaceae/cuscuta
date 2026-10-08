use std::sync::OnceLock;

use cuscuta_common::{
    api::xxxxxx::XxxxxxUrl,
    data::{AppVersionData, SongsWithHash},
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
    pub worker_empty_friends_delay_time_secs: u64,
}

pub static XXXXXX_URL: OnceLock<RwLock<Option<XxxxxxUrl>>> = OnceLock::new();
pub static APP_VERSION_DATA: OnceLock<RwLock<Option<AppVersionData>>> = OnceLock::new();
pub static SONG_LIST: OnceLock<RwLock<Option<Vec<SongsWithHash>>>> = OnceLock::new();
pub static CONFIG: OnceLock<RwLock<Option<Config>>> = OnceLock::new();

pub static ACCOUNT_ROW: OnceLock<RwLock<Option<AccountRow>>> = OnceLock::new();

pub static WORKER_ID: OnceLock<String> = OnceLock::new();

#[cfg(test)]
pub mod mock {
    use cuscuta_test::mock::SimpleMockable;

    use crate::data::Config;

    impl SimpleMockable for Config {
        fn mock() -> Self {
            Self {
                worker_max_jobs: 8,
                worker_max_retry_count: 5,
                worker_exponential_backoff_base_millis: 100,
                worker_exponential_backoff_multiplier: 2,
                worker_exponential_backoff_max_delay_millis: 500,
                redis_stream_refresh_ttl: 600,
                worker_account_lease_time_secs: 60,
                _worker_account_lease_time_refresh_gap_secs: 5,
                worker_job_max_work_time_secs: 600,
                worker_empty_friends_delay_time_secs: 10,
            }
        }
    }
}
