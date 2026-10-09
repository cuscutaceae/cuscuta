use chrono::{DateTime, Utc};
use cuscuta_common::{
    api::{fetch_json, xxxxxx::XxxxxxUrl},
    data::{AppVersionData, ScirpophagaData},
    db::account::update_account_lease_time,
    quick_fetch::QuickFetch,
};
use tokio_util::sync::CancellationToken;

use crate::{
    config::fetch_env,
    data::{ACCOUNT_ROW, APP_VERSION_DATA, XXXXXX_URL},
    db::postgresql::try_open_transaction,
};

pub async fn sync_scirpophaga_data(_: &CancellationToken) {
    async fn try_sync() -> Result<XxxxxxUrl, String> {
        let config = fetch_env();
        let scirpophaga_data = if config.use_online_prefix {
            fetch_json(
                &config
                    .scirpophaga_url
                    .clone()
                    .expect("USE_ONLINE_PREFIX is true, but SCIRPOPHAGA_URL is not set"),
            )
            .await
            .map_err(|e| format!("failed to fetch from SCIRPOPHAGA_URL: {e}"))?
        } else {
            ScirpophagaData {
                c2: "useless".to_owned(),
                common_path: config
                    .api_prefix_common
                    .clone()
                    .expect("USE_ONLINE_PREFIX is false, but API_PREFIX_COMMON is not set"),
                auth_path: config
                    .api_prefix_auth
                    .clone()
                    .expect("USE_ONLINE_PREFIX is false, but API_PREFIX_AUTH is not set"),
            }
        };
        let common_prefix = scirpophaga_data.common_path.trim_end_matches('/');
        let auth_prefix = scirpophaga_data.auth_path.trim_end_matches('/');
        let xxxxxx_url = XxxxxxUrl {
            login: format!("{auth_prefix}{}", config.api_endpoint_login),
            list_friend: format!("{common_prefix}{}", config.api_endpoint_list_friends),
            add_friend: format!("{common_prefix}{}", config.api_endpoint_add_friends),
            delete_friend: format!("{common_prefix}{}", config.api_endpoint_delete_friends),
            get_rank: format!("{common_prefix}{}", config.api_endpoint_get_rank),
            get_notification: format!("{common_prefix}{}", config.api_endpoint_notification),
            compose: format!("{common_prefix}{}", config.api_endpoint_compose_aggregate),
            chilo: config.api_chilo.clone(),
        };
        XXXXXX_URL
            .try_write(|_| xxxxxx_url.clone().into())
            .map_err(|e| format!("failed to write SCIRPOPHAGA_DATA: {e}"))?;
        Ok(xxxxxx_url)
    }
    let retries = fetch_env().resource_update_retries;
    for retry in 0..retries {
        tracing::info!("sync_scirpophaga_data: trying sync scirpophaga data... {retry}/{retries}");
        match try_sync().await {
            Ok(data) => {
                tracing::info!("sync_scirpophaga_data: scirpophaga data initialized: {data:?}",);
                return;
            }
            Err(e) => {
                tracing::error!("sync_scirpophaga_data: failed to sync scirpophaga data: {e}");
            }
        }
    }
}

pub async fn sync_app_version_data(_: &CancellationToken) {
    async fn try_sync() -> Result<AppVersionData, String> {
        let config = fetch_env();
        let app_version_data = if config.resources_app_version_use_online {
            fetch_json::<AppVersionData>(&config.resources_app_version_url.clone().expect(
                "RESOURCES_APP_VERSION_USE_ONLINE is true but RESOURCES_APP_VERSION_URL is not set",
            ))
            .await
            .map_err(|e| format!("failed to fetch bundle data from url: {e}"))?
        } else {
            let version_number = config.resources_app_version_default.clone().expect(
                "RESOURCES_APP_VERSION_USE_ONLINE is false but RESOURCES_APP_VERSION_DEFAULT is not set",
            );
            if version_number.is_empty() {
                return Err("invalid version number: empty string".to_owned());
            }
            AppVersionData {
                version: version_number,
            }
        };
        APP_VERSION_DATA
            .try_write(|_| app_version_data.clone().into())
            .map_err(|e| format!("failed to write APP_VERSION_DATA: {e}"))?;
        Ok(app_version_data)
    }
    let retries = fetch_env().resource_update_retries;
    for retry in 0..retries {
        tracing::info!("sync_app_version_data: trying sync bundle data... {retry}/{retries}");
        match try_sync().await {
            Ok(app_version_data) => {
                tracing::info!(
                    "sync_app_version_data: bundle data initialized: appVer:{}",
                    app_version_data.version,
                );
                return;
            }
            Err(e) => {
                tracing::error!("sync_app_version_data: failed to sync bundle data: {e}");
            }
        }
    }
}

pub async fn update_lease_time(_: &CancellationToken) {
    async fn try_update() -> Result<Option<DateTime<Utc>>, String> {
        let transaction = try_open_transaction()
            .await
            .map_err(|e| format!("failed to open transaction: {e}"))?;
        let account_row = ACCOUNT_ROW
            .read_spinning(std::clone::Clone::clone)
            .await
            .map_err(|e| format!("failed to fetch account_row, is account not logged yet? {e}"))?;
        let config = fetch_env();
        let lease_time = update_account_lease_time(
            transaction,
            &account_row,
            config.worker_account_lease_time_secs,
        )
        .await
        .map_err(|e| format!("failed to update lease time: {e}"))?;
        Ok(lease_time)
    }
    tracing::debug!("update_lease_time: trying update lease time");
    match try_update().await {
        Ok(lease_time) => {
            if let Some(lease_time) = lease_time {
                tracing::debug!("update_lease_time: updated lease time: {lease_time}");
            } else {
                tracing::warn!(
                    "update_lease_time: failed to update account lease time: no returns found, id may not exist"
                );
            }
        }
        Err(e) => {
            tracing::warn!("update_lease_time: failed to update account lease time: {e}");
        }
    }
}
