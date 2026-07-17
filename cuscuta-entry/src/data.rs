use std::sync::OnceLock;

use cuscuta_common::data::{BundleData, Song};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct Config {
    pub redis_stream_refresh_ttl: i64,
}

pub static CONFIG: OnceLock<RwLock<Option<Config>>> = OnceLock::new();

pub static BUNDLE_DATA: OnceLock<RwLock<Option<BundleData>>> = OnceLock::new();
pub static SONG_LIST: OnceLock<RwLock<Option<Vec<Song>>>> = OnceLock::new();
