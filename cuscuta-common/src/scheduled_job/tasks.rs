use std::{pin::Pin, sync::OnceLock};

use redis::TypedCommands;
use sha2::Digest;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::{
    api::fetch_json,
    config::{
        IntoPostgresqlUrlAddress, IntoRedisUrlAddress, IntoResourcesUpdatePeriodAndRetries,
        IntoSongUrlAddress,
    },
    data::{Song, SongsResult, SongsWithHash},
    quick_fetch::QuickFetch,
};

type QuickFetchType<T> = OnceLock<RwLock<Option<T>>>;

fn calc_song_list_hash(song_list: &[Song]) -> String {
    struct SongPart<'a> {
        idx: i32,
        id: &'a str,
        difficulty: u8,
    }
    let mut parts = song_list
        .iter()
        .map(|it| {
            let difficulty = it
                .difficulties
                .iter()
                .fold(0u8, |b, d| b | (1 << d.rating_class));
            SongPart {
                idx: it.idx,
                id: &it.id,
                difficulty,
            }
        })
        .collect::<Vec<_>>();
    parts.sort_by_key(|it| it.idx);
    let data = parts
        .into_iter()
        .flat_map(
            |SongPart {
                 idx,
                 id,
                 difficulty,
             }| {
                idx.to_le_bytes()
                    .iter()
                    .chain(id.as_bytes())
                    .chain(&[difficulty])
                    .copied()
                    .collect::<Vec<_>>()
            },
        )
        .collect::<Vec<_>>();
    let arr = sha2::Sha256::digest(data)
        .into_iter()
        .take(8)
        .collect::<Vec<_>>();
    hex::encode(&arr)
}

/// 同步歌曲列表
pub fn sync_song_list<T>(
    global_song_list: &'static QuickFetchType<Vec<SongsWithHash>>,
    config: &T,
) -> impl Fn(&CancellationToken) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>
where
    T: IntoSongUrlAddress + IntoResourcesUpdatePeriodAndRetries,
{
    move |_| {
        let song_list_url = config.song_list_url_address().to_string();
        let max_retries = config.resource_update_retries();
        Box::pin(async move {
            async fn try_sync(
                global_song_list: &QuickFetchType<Vec<SongsWithHash>>,
                song_list_url: &str,
            ) -> Result<(usize, usize, String), String> {
                let song_list: Vec<_> = fetch_json::<SongsResult>(song_list_url)
                    .await
                    .map_err(|e| format!("failed to fetch song data from url: {e}"))
                    .map(|it| it.songs.into_iter().filter_map(Option::<Song>::from))?
                    .collect();
                let music_len = song_list.len();
                let chart_len = song_list.iter().fold(0, |v, it| v + it.difficulties.len());
                tracing::info!("sync_song_list: hashing");
                let hash = calc_song_list_hash(&song_list);
                global_song_list
                    .try_write(|songs_with_hash| {
                        let needs_add = songs_with_hash
                            .as_ref()
                            .is_none_or(|it| it.first().is_none_or(|it| it.hash != hash));
                        if !needs_add {
                            tracing::info!("sync_song_list: hash did not change, discard update");
                            return songs_with_hash;
                        }
                        let new = SongsWithHash {
                            hash: hash.clone(),
                            songs: song_list.clone(),
                        };
                        match songs_with_hash {
                            Some(mut songs_with_hash) => {
                                songs_with_hash.insert(0, new);
                                Some(songs_with_hash)
                            }
                            None => Some(vec![new]),
                        }
                    })
                    .map_err(|e| format!("failed to write SONG_LIST: {e}"))?;
                Ok((music_len, chart_len, hash))
            }
            for retry in 0..max_retries {
                tracing::info!("sync_song_list: trying sync song list... {retry}/{max_retries}");
                match try_sync(global_song_list, &song_list_url).await {
                    Ok((music_len, chart_len, hash)) => {
                        tracing::info!(
                            "sync_song_list: song list initialized (music:{music_len}, charts:{chart_len}, hash: {hash})"
                        );
                        return;
                    }
                    Err(e) => tracing::error!("sync_song_list: failed to sync song list: {e}"),
                }
            }
        })
    }
}

/// 打开Postgresql pool
pub fn open_postgresql_client<T>(
    global_postgresql_pool: &'static QuickFetchType<sqlx::PgPool>,
    config: &T,
) -> impl Fn(&CancellationToken) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>
where
    T: IntoPostgresqlUrlAddress,
{
    move |_| {
        let addr = config.postgresql_address().to_string();
        Box::pin(async move {
            async fn try_connect(
                global_postgresql_pool: &QuickFetchType<sqlx::PgPool>,
                addr: &str,
            ) -> Result<(), String> {
                tracing::debug!("postgresql_open: {addr}");
                let x = PgPoolOptions::new()
                    .max_connections(5)
                    .connect(addr)
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
            if let Err(e) = try_connect(global_postgresql_pool, &addr).await {
                tracing::error!("postgresql_open: failed to connect to postgresql server: {e}");
                return;
            }
            tracing::info!("postgresql_open: postgresql pool created successfully");
        })
    }
}

/// 打开redis client
pub fn open_redis_client<T>(
    global_redis_client: &'static OnceLock<redis::Client>,
    config: &T,
) -> impl Fn(&CancellationToken) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>
where
    T: IntoRedisUrlAddress,
{
    move |_| {
        let addr = config.redis_address().to_string();
        Box::pin(async move {
            fn try_connect(
                global_redis_client: &OnceLock<redis::Client>,
                addr: &str,
            ) -> Result<(), String> {
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
            if let Err(e) = try_connect(global_redis_client, &addr) {
                tracing::error!("redis_open: failed to connect to redis server: {e}");
                return;
            }
            tracing::info!("redis_open: redis client created successfully");
        })
    }
}
