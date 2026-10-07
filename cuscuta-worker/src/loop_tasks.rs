use std::{env, str::FromStr};

use chrono::{DateTime, Utc};
use cuscuta_common::{
    api::{fetch_env_as_json, xxxxxx::XxxxxxUrl},
    data::{AppVersionData, ScirpophagaData, Song, SongsResult},
    db::account::update_account_lease_time,
    quick_fetch::QuickFetch,
};
use redis::TypedCommands;
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;

use crate::{
    data::{ACCOUNT_ROW, APP_VERSION_DATA, CONFIG, Config, SONG_LIST, XXXXXX_URL},
    db::{
        postgresql::{POSTGRESQL_POOL, try_open_transaction},
        redis::REDIS_CLIENT,
    },
};

pub async fn sync_scirpophaga_data(_: &CancellationToken) {
    fn try_get_env_var(env: &str) -> Result<String, String> {
        env::var(env)
            .map_err(|e| format!("failed to fetch env var: {env}: {e}"))
            .map(|it| format!("/{}", it.trim_start_matches('/')))
    }
    async fn try_sync() -> Result<XxxxxxUrl, String> {
        let use_online_prefix =
            env::var("USE_ONLINE_PREFIX").map_or(true, |it| it.parse::<bool>().unwrap_or(true));
        let scirpophaga_data = if use_online_prefix {
            fetch_env_as_json("SCIRPOPHAGA_URL")
                .await
                .map_err(|e| format!("failed to fetch from SCIRPOPHAGA_URL: {e}"))?
        } else {
            ScirpophagaData {
                c2: "useless".to_owned(),
                common_path: env::var("API_PREFIX_COMMON")
                    .map_err(|e| format!("failed to read API_PREFIX_COMMON: {e}"))?,
                auth_path: env::var("API_PREFIX_AUTH")
                    .map_err(|e| format!("failed to read API_PREFIX_AUTH: {e}"))?,
            }
        };
        let common_prefix = scirpophaga_data.common_path.trim_end_matches('/');
        let auth_prefix = scirpophaga_data.auth_path.trim_end_matches('/');
        let xxxxxx_url = XxxxxxUrl {
            login: format!("{auth_prefix}{}", try_get_env_var("API_ENDPOINT_LOGIN")?),
            list_friend: format!(
                "{common_prefix}{}",
                try_get_env_var("API_ENDPOINT_LIST_FRIENDS")?
            ),
            add_friend: format!(
                "{common_prefix}{}",
                try_get_env_var("API_ENDPOINT_ADD_FRIENDS")?
            ),
            delete_friend: format!(
                "{common_prefix}{}",
                try_get_env_var("API_ENDPOINT_DELETE_FRIENDS")?
            ),
            get_rank: format!(
                "{common_prefix}{}",
                try_get_env_var("API_ENDPOINT_GET_RANK")?
            ),
            get_notification: format!(
                "{common_prefix}{}",
                try_get_env_var("API_ENDPOINT_NOTIFICATION")?
            ),
            compose: format!(
                "{common_prefix}{}",
                try_get_env_var("API_ENDPOINT_COMPOSE_AGGREGATE")?
            ),
        };
        XXXXXX_URL
            .try_write(|_| xxxxxx_url.clone().into())
            .map_err(|e| format!("failed to write SCIRPOPHAGA_DATA: {e}"))?;
        Ok(xxxxxx_url)
    }
    if XXXXXX_URL.is_initialized() {
        tracing::trace!("scirpophaga data sync");
        return;
    }
    tracing::info!("sync_scirpophaga_data: trying sync scirpophaga data");
    match try_sync().await {
        Ok(data) => {
            tracing::info!("sync_scirpophaga_data: scirpophaga data initialized: {data:?}",);
        }
        Err(e) => {
            tracing::error!("sync_scirpophaga_data: failed to sync scirpophaga data: {e}");
        }
    }
}

pub async fn sync_app_version_data(_: &CancellationToken) {
    async fn try_sync() -> Result<AppVersionData, String> {
        let use_online_version = env::var("RESOURCES_APP_VERSION_USE_ONLINE")
            .map_or(true, |it| it.parse::<bool>().unwrap_or(true));
        let app_version_data = if use_online_version {
            fetch_env_as_json::<AppVersionData>("RESOURCES_APP_VERSION_URL")
                .await
                .map_err(|e| format!("failed to fetch bundle data from url: {e}"))?
        } else {
            let version_number = env::var("RESOURCES_APP_VERSION_DEFAULT")
                .map_err(|e| format!("failed to read env: RESOURCES_APP_VERSION_DEFAULT: {e}"))?;
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
    if APP_VERSION_DATA.is_initialized() {
        tracing::trace!("app version data sync");
        return;
    }
    tracing::info!("sync_app_version_data: trying sync bundle data");
    match try_sync().await {
        Ok(app_version_data) => {
            tracing::info!(
                "sync_app_version_data: bundle data initialized: appVer:{}",
                app_version_data.version,
            );
        }
        Err(e) => {
            tracing::error!("sync_app_version_data: failed to sync bundle data: {e}");
        }
    }
}

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
            worker_max_jobs: read_as_number("WORKER_MAX_JOBS")?,
            worker_max_retry_count: read_as_number("WORKER_MAX_RETRIES")?,
            worker_exponential_backoff_base_millis: read_as_number(
                "WORKER_EXPONENTIAL_BACKOFF_BASE_MILLIS",
            )?,
            worker_exponential_backoff_multiplier: read_as_number(
                "WORKER_EXPONENTIAL_BACKOFF_MULTIPLIER",
            )?,
            worker_exponential_backoff_max_delay_millis: read_as_number(
                "WORKER_EXPONENTIAL_BACKOFF_MAX_DELAY_MILLIS",
            )?,
            redis_stream_refresh_ttl: read_as_number("REDIS_STREAM_REFRESH_TTL")?,
            worker_account_lease_time_secs: read_as_number("WORKER_ACCOUNT_LEASE_TIME_SECS")?,
            _worker_account_lease_time_refresh_gap_secs: read_as_number(
                "WORKER_ACCOUNT_LEASE_TIME_REFRESH_GAP_SECS",
            )?,
            worker_job_max_work_time_secs: read_as_number("WORKER_JOB_MAX_WORK_TIME_SECS")?,
            worker_empty_friends_delay_time_secs: read_as_number(
                "WORKER_EMPTY_FRIENDS_DELAY_TIME_SECS",
            )?,
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

pub async fn sync_song_list(_: &CancellationToken) {
    async fn try_sync() -> Result<(usize, usize), String> {
        let song_list: Vec<_> = fetch_env_as_json::<SongsResult>("RESOURCES_SONG_URL")
            .await
            .map_err(|e| format!("failed to fetch song data from url: {e}"))
            .map(|it| it.songs.into_iter().filter_map(Option::<Song>::from))?
            .collect();
        let music_len = song_list.len();
        let chart_len = song_list.iter().fold(0, |v, it| v + it.difficulties.len());
        SONG_LIST
            .try_write(move |_| song_list.into())
            .map_err(|e| format!("failed to write SONG_LIST: {e}"))?;
        Ok((music_len, chart_len))
    }
    if SONG_LIST.is_initialized() {
        tracing::trace!("song list sync");
        return;
    }
    tracing::info!("sync_song_list: trying sync song list");
    match try_sync().await {
        Ok((music_len, chart_len)) => tracing::info!(
            "sync_song_list: song list initialized (music:{music_len}, charts:{chart_len})"
        ),
        Err(e) => tracing::error!("sync_song_list: failed to sync song list: {e}"),
    }
}

pub async fn open_redis_client(_: &CancellationToken) {
    fn try_connect() -> Result<(), String> {
        let addr = env::var("REDIS_ADDR").map_err(|_| "failed to read env: REDIS_ADDR")?;
        tracing::debug!("redis_open: redis: {addr}");
        let redis = redis::Client::open(addr)
            .map_err(|e| format!("failed to open redis client(phase 1): {e}"))?;
        let mut con = redis
            .get_connection()
            .map_err(|e| format!("failed to open redis client(phase 2): {e}"))?;
        con.ping()
            .map_err(|e| format!("failed to open redis client(phase 3): {e}"))?;
        REDIS_CLIENT
            .set(redis)
            .map_err(|_| "failed to set redis client".to_string())?;
        Ok(())
    }
    if REDIS_CLIENT.get().is_some() {
        return;
    }
    tracing::debug!("redis_open: trying to connect to redis server...");
    if let Err(e) = try_connect() {
        tracing::error!("redis_open: failed to connect to redis server: {e}");
        return;
    }
    tracing::info!("redis_open: redis client created successfully");
}

pub async fn open_postgresql_client(_: &CancellationToken) {
    async fn try_connect() -> Result<(), String> {
        let addr = env::var("ACCOUNTS_SQL_ADDR")
            .map_err(|e| format!("failed to read ACCOUNTS_SQL_ADDR: {e}"))?;
        tracing::debug!("postgresql_open: {addr}");
        let x = PgPoolOptions::new()
            .max_connections(5)
            .connect(addr.as_str())
            .await
            .map_err(|e| format!("failed to connect to postgresql server: {e}"))?;
        POSTGRESQL_POOL
            .try_write(move |_| x.into())
            .map_err(|e| format!("failed to write postgresql pool: {e}"))?;
        Ok(())
    }
    if POSTGRESQL_POOL.is_initialized() {
        return;
    }
    tracing::debug!("postgresql_open: trying to connect to postgresql server...");
    if let Err(e) = try_connect().await {
        tracing::error!("postgresql_open: failed to connect to postgresql server: {e}");
        return;
    }
    tracing::info!("postgresql_open: postgresql pool created successfully");
}

pub async fn update_lease_time(_: &CancellationToken) {
    async fn try_update() -> Result<Option<DateTime<Utc>>, String> {
        let transaction = try_open_transaction()
            .await
            .map_err(|e| format!("failed to open transaction: {e}"))?;
        let account_row = ACCOUNT_ROW
            .try_read(std::clone::Clone::clone)
            .map_err(|e| format!("failed to fetch account_row, is account not logged yet? {e}"))?;
        let config = CONFIG
            .try_read(std::clone::Clone::clone)
            .map_err(|e| format!("failed to read config: {e}"))?;
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
