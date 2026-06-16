//! cuscuta的entry
//!
//! # 依赖环境变量
//! cuscuta-entry使用环境变量注入参数，这个crate依赖的环境变量有：
//! - `REDIS_STREAM_REFRESH_TTL`
//! - `GITHUB_BUNDLE_REPOSITORY`
//! - `GITHUB_BUNDLE_PATH`
//! - `GITHUB_BUNDLE_TOKEN`
//! - `GITHUB_SONG_REPOSITORY`
//! - `GITHUB_SONG_PATH`
//! - `GITHUB_SONG_TOKEN`
//! - `REDIS_ADDR`
//! - `ACCOUNTS_SQL_ADDR`

#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]

mod data;
mod db;
mod endpoints;
mod init;
mod loop_tasks;

use crate::{
    data::{BUNDLE_DATA, CONFIG, SONG_LIST},
    db::{postgresql::POSTGRESQL_POOL, redis::REDIS_CLIENT},
    endpoints::query::query,
    enqueue::enqueue,
    loop_tasks::sync_config,
};
use cuscuta_common::quick_fetch::QuickFetch;

use axum::{
    Json, Router,
    response::IntoResponse,
    routing::{get, post},
};
use cuscuta_common::{
    batch_check_initialized,
    scheduled_job::{register_individual_job, register_job},
};
use reqwest::StatusCode;
use serde_json::json;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::{
    endpoints::enqueue,
    init::cuscuta_init,
    loop_tasks::{open_postgresql_client, open_redis_client, sync_bundle_data, sync_song_list},
};

#[tokio::main]
async fn main() {
    env_logger::init();
    log::info!("starting...");
    let halt_token = CancellationToken::new();
    let service = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/v1/enqueue", post(enqueue))
        .route("/v1/query", get(query));
    let addr = TcpListener::bind("0.0.0.0:8081")
        .await
        .expect("failed to bind 0.0.0.0:8081");
    log::info!("listening in 0.0.0.0:8081...");
    tokio::spawn(register_individual_job(
        halt_token.clone(),
        CancellationToken::new(),
        10,
        cuscuta_init,
    ));
    tokio::spawn(register_job(halt_token.clone(), 10, open_redis_client));
    tokio::spawn(register_job(halt_token.clone(), 10, open_postgresql_client));
    tokio::spawn(register_job(halt_token.clone(), 10, sync_bundle_data));
    tokio::spawn(register_job(halt_token.clone(), 10, sync_song_list));
    tokio::spawn(register_job(halt_token.clone(), 10, sync_config));
    axum::serve(addr, service)
        .with_graceful_shutdown(shutdown_signal(halt_token))
        .await
        .unwrap_or_else(|e| panic!("{e:?}"));
}

fn check_ready() -> Option<&'static str> {
    if REDIS_CLIENT.get().is_none() {
        return Some("redis client is not initialized");
    }
    batch_check_initialized!(CONFIG, BUNDLE_DATA, SONG_LIST, POSTGRESQL_POOL);
    None
}

async fn readyz() -> impl IntoResponse {
    check_ready().map_or_else(
        || (StatusCode::OK, Json(json!({"status":"ready"}))),
        |e| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"status": "not ready", "reason": e})),
            )
        },
    )
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({"health":"ok"})))
}

#[allow(clippy::ignored_unit_patterns)]
async fn shutdown_signal(cancellation_token: CancellationToken) {
    #[cfg(target_os = "linux")]
    {
        use tokio::signal::{
            self,
            unix::{SignalKind, signal},
        };
        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = signal::ctrl_c() => {},
            _ = sigterm.recv() => {},
            _ = cancellation_token.cancelled() => {},
        }
    }
    #[cfg(target_os = "windows")]
    {
        use tokio::signal;
        log::info!("咱其实挺想知道谁会在Windows上跑这个的……");
        tokio::select! {
            _ = signal::ctrl_c() => {},
            _ = cancellation_token.cancelled() => {},
        }
    }
    log::error!("cuscuta-entry halting...");
    cancellation_token.cancel();
}
