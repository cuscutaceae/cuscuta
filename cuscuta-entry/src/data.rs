use std::sync::OnceLock;

use cuscuta_common::data::SongsWithHash;
use tokio::sync::RwLock;

pub static SONG_LIST: OnceLock<RwLock<Option<Vec<SongsWithHash>>>> = OnceLock::new();
