use std::sync::OnceLock;

use cuscuta_common::{
    api::xxxxxx::XxxxxxUrl,
    data::{AppVersionData, SongsWithHash},
    db::account::AccountRow,
};
use tokio::sync::RwLock;

pub static XXXXXX_URL: OnceLock<RwLock<Option<XxxxxxUrl>>> = OnceLock::new();
pub static APP_VERSION_DATA: OnceLock<RwLock<Option<AppVersionData>>> = OnceLock::new();
pub static SONG_LIST: OnceLock<RwLock<Option<Vec<SongsWithHash>>>> = OnceLock::new();

pub static ACCOUNT_ROW: OnceLock<RwLock<Option<AccountRow>>> = OnceLock::new();

pub static WORKER_ID: OnceLock<String> = OnceLock::new();
