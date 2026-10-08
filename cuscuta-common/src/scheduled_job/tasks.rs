use std::{env, pin::Pin, sync::OnceLock};

use redis::TypedCommands;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::{
    api::fetch_env_as_json,
    data::{Song, SongsResult},
    quick_fetch::QuickFetch,
};

type QuickFetchType<T> = OnceLock<RwLock<Option<T>>>;

/// 同步歌曲列表
pub fn sync_song_list(
    global_song_list: &'static QuickFetchType<Vec<Song>>,
) -> impl Fn(&CancellationToken) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
    |_| {
        Box::pin(async {
            async fn try_sync(
                global_song_list: &QuickFetchType<Vec<Song>>,
            ) -> Result<(usize, usize), String> {
                let song_list: Vec<_> = fetch_env_as_json::<SongsResult>("RESOURCES_SONG_URL")
                    .await
                    .map_err(|e| format!("failed to fetch song data from url: {e}"))
                    .map(|it| it.songs.into_iter().filter_map(Option::<Song>::from))?
                    .collect();
                let music_len = song_list.len();
                let chart_len = song_list.iter().fold(0, |v, it| v + it.difficulties.len());
                global_song_list
                    .try_write(move |_| song_list.into())
                    .map_err(|e| format!("failed to write SONG_LIST: {e}"))?;
                Ok((music_len, chart_len))
            }
            if global_song_list.is_initialized() {
                tracing::trace!("song list sync");
                return;
            }
            tracing::info!("sync_song_list: trying sync song list");
            match try_sync(global_song_list).await {
                Ok((music_len, chart_len)) => tracing::info!(
                    "sync_song_list: song list initialized (music:{music_len}, charts:{chart_len})"
                ),
                Err(e) => tracing::error!("sync_song_list: failed to sync song list: {e}"),
            }
        })
    }
}

/// 打开Postgresql pool
pub fn open_postgresql_client(
    global_postgresql_pool: &'static QuickFetchType<sqlx::PgPool>,
) -> impl Fn(&CancellationToken) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
    |_| {
        Box::pin(async {
            async fn try_connect(
                global_postgresql_pool: &QuickFetchType<sqlx::PgPool>,
            ) -> Result<(), String> {
                let addr = env::var("ACCOUNTS_SQL_ADDR")
                    .map_err(|e| format!("failed to read ACCOUNTS_SQL_ADDR: {e}"))?;
                tracing::debug!("postgresql_open: {addr}");
                let x = PgPoolOptions::new()
                    .max_connections(5)
                    .connect(addr.as_str())
                    .await
                    .map_err(|e| format!("failed to connect to postgresql server: {e}"))?;
                global_postgresql_pool
                    .try_write(move |_| x.into())
                    .map_err(|e| format!("failed to write postgresql pool: {e}"))?;
                Ok(())
            }
            if global_postgresql_pool.is_initialized() {
                return;
            }
            tracing::debug!("postgresql_open: trying to connect to postgresql server...");
            if let Err(e) = try_connect(global_postgresql_pool).await {
                tracing::error!("postgresql_open: failed to connect to postgresql server: {e}");
                return;
            }
            tracing::info!("postgresql_open: postgresql pool created successfully");
        })
    }
}

/// 打开redis client
pub fn open_redis_client(
    global_redis_client: &'static OnceLock<redis::Client>,
) -> impl Fn(&CancellationToken) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
    |_| {
        Box::pin(async {
            fn try_connect(global_redis_client: &OnceLock<redis::Client>) -> Result<(), String> {
                let addr = env::var("REDIS_ADDR").map_err(|_| "failed to read env: REDIS_ADDR")?;
                tracing::debug!("redis_open: redis: {addr}");
                let redis = redis::Client::open(addr)
                    .map_err(|e| format!("failed to open redis client(phase 1): {e}"))?;
                let mut con = redis
                    .get_connection()
                    .map_err(|e| format!("failed to open redis client(phase 2): {e}"))?;
                con.ping()
                    .map_err(|e| format!("failed to open redis client(phase 3): {e}"))?;
                global_redis_client
                    .set(redis)
                    .map_err(|_| "failed to set redis client".to_string())?;
                Ok(())
            }
            if global_redis_client.get().is_some() {
                return;
            }
            tracing::debug!("redis_open: trying to connect to redis server...");
            if let Err(e) = try_connect(global_redis_client) {
                tracing::error!("redis_open: failed to connect to redis server: {e}");
                return;
            }
            tracing::info!("redis_open: redis client created successfully");
        })
    }
}
