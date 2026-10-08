use cuscuta_common::{data::read_parsed_env, quick_fetch::QuickFetch};
use tokio_util::sync::CancellationToken;

use crate::data::{CONFIG, Config};

pub async fn sync_config(_: &CancellationToken) {
    fn try_sync() -> Result<(), String> {
        let config = Config {
            redis_stream_refresh_ttl: read_parsed_env("REDIS_STREAM_REFRESH_TTL")?,
            enable_stat: read_parsed_env::<bool>("STAT_ENABLE")?,
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
