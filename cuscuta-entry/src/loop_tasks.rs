use std::{env, str::FromStr};

use cuscuta_common::{
    api::fetch_env_as_json,
    data::{Song, SongsResult},
    quick_fetch::QuickFetch,
};
use redis::TypedCommands;
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;

use crate::{
    data::{CONFIG, Config, SONG_LIST},
    db::{postgresql::POSTGRESQL_POOL, redis::REDIS_CLIENT},
};

pub async fn sync_config(_: &CancellationToken) {
    fn read_as_number<T>(key: &str) -> Result<T, String>
    where
        T: FromStr,
    {
        env::var(key)
            .map_err(|e| format!("failed to read {key}: {e}"))?
            .parse::<T>()
            .map_err(|_| format!("failed to read {key}: failed to parse"))
    }
    fn try_sync() -> Result<(), String> {
        let config = Config {
            redis_stream_refresh_ttl: read_as_number("REDIS_STREAM_REFRESH_TTL")?,
            enable_stat: read_as_number::<bool>("STAT_ENABLE")?,
        };
        CONFIG
            .try_write(move |_| config.into())
            .map_err(|e| format!("failed to write CONFIG: {e}"))?;
        Ok(())
    }
    if CONFIG.is_initialized() {
        tracing::trace!("config sync");
        return;
    }
    tracing::info!("sync_config: trying sync config");
    if let Err(e) = try_sync() {
        tracing::error!("sync_config: failed to sync config: {e}");
        return;
    }
    tracing::info!("sync_config: config initialized");
}
