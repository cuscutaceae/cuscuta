use std::env;

use cuscuta_common::{
    api::read_env_url_and_fetch_json, data::ScirpophagaData, quick_fetch::QuickFetch,
    scheduled_job::tasks::get_data_fetch_max_retries,
};
use tokio_util::sync::CancellationToken;

use crate::C2;

pub async fn sync_scirpophaga_data(_: &CancellationToken) {
    async fn try_sync() -> Result<String, String> {
        let use_online_key =
            env::var("USE_ONLINE_KEY").map_or(true, |it| it.parse::<bool>().unwrap_or(true));
        let data = if use_online_key {
            read_env_url_and_fetch_json::<ScirpophagaData>("SCIRPOPHAGA_URL")
                .await
                .map_err(|e| format!("failed to fetch from SCIRPOPHAGA_URL: {e}"))?
                .c2
        } else {
            env::var("BIN_C2").map_err(|e| format!("failed to read API_PREFIX_COMMON: {e}"))?
        };
        let vec = hex::decode(data.clone())
            .map_err(|e| format!("failed to decode string: {data} ({e})"))?;
        C2.try_write(|_| Some(vec))
            .map_err(|e| format!("failed to write C2: {e}"))?;
        Ok(data)
    }
    let retries = get_data_fetch_max_retries();
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
