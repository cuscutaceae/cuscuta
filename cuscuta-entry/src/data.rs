use std::sync::OnceLock;

use cuscuta_common::data::SongsWithHash;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct Config {
    pub redis_stream_refresh_ttl: i64,
    pub enable_stat: bool,
}

pub static CONFIG: OnceLock<RwLock<Option<Config>>> = OnceLock::new();

pub static SONG_LIST: OnceLock<RwLock<Option<Vec<SongsWithHash>>>> = OnceLock::new();
