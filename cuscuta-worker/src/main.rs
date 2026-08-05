//! cuscuta的worker
//!
//! # 依赖环境变量
//! cuscuta-worker使用环境变量注入参数，这个crate依赖的环境变量有：
//! - `WORKER_MAX_JOBS`
//! - `WORKER_MAX_RETRIES`
//! - `WORKER_EXPONENTIAL_BACKOFF_BASE_MILLIS`
//! - `WORKER_EXPONENTIAL_BACKOFF_MULTIPLIER`
//! - `WORKER_EXPONENTIAL_BACKOFF_MAX_DELAY_MILLIS`
//! - `WORKER_ACCOUNT_LEASE_TIME_SECS`
//! - `WORKER_ACCOUNT_LEASE_TIME_REFRESH_GAP_SECS`
//! - `WORKER_EMPTY_FRIENDS_DELAY_TIME_SECS`
//! - `REDIS_STREAM_REFRESH_TTL`
//! - `GITHUB_BUNDLE_REPOSITORY`
//! - `GITHUB_BUNDLE_PATH`
//! - `GITHUB_BUNDLE_TOKEN`
//! - `GITHUB_SONG_REPOSITORY`
//! - `GITHUB_SONG_PATH`
//! - `GITHUB_SONG_TOKEN`
//! - `REDIS_ADDR`
//! - `ACCOUNTS_SQL_ADDR`
//! - `API_CHILO`
//! - `API_LOGIN`
//! - `API_LIST_FRIENDS`
//! - `API_ADD_FRIENDS`
//! - `API_DELETE_FRIENDS`
//! - `API_GET_RANK`
//!

mod data;
mod db;
mod init;
mod loop_tasks;
mod worker;

use std::env;

use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
use cuscuta_common::{
    batch_check_initialized,
    db::account::try_release_account,
    quick_fetch::QuickFetch,
    scheduled_job::{register_individual_job, register_job},
};
use serde_json::json;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::{
    data::{ACCOUNT_ROW, BUNDLE_DATA, CONFIG, SONG_LIST},
    db::{
        postgresql::{POSTGRESQL_POOL, try_open_transaction},
        redis::REDIS_CLIENT,
    },
    init::cuscuta_init,
    loop_tasks::{
        open_postgresql_client, open_redis_client, sync_bundle_data, sync_config, sync_song_list,
        update_lease_time,
    },
    worker::{resume_state, worker_loop},
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    tracing::info!("starting...");
    let halt_token = CancellationToken::new();
    let service = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz));
    let addr = TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("failed to bind 0.0.0.0:8080");
    tracing::info!("listening in 0.0.0.0:8080...");
    tokio::spawn(register_individual_job(
        halt_token.clone(),
        CancellationToken::new(),
        10,
        cuscuta_init,
    ));
    tokio::spawn(register_job(halt_token.clone(), 10, open_redis_client));
    tokio::spawn(register_job(halt_token.clone(), 10, open_postgresql_client));
    tokio::spawn(register_job(halt_token.clone(), 10, sync_config));
    tokio::spawn(register_job(halt_token.clone(), 10, sync_bundle_data));
    tokio::spawn(register_job(halt_token.clone(), 10, sync_song_list));
    tokio::spawn(register_job(
        halt_token.clone(),
        env::var("WORKER_ACCOUNT_LEASE_TIME_REFRESH_GAP_SECS")
            .map_err(|_| ())
            .and_then(|it| it.parse().map_err(|_| ()))
            .unwrap_or(30),
        update_lease_time,
    ));
    tokio::spawn(start_loop(halt_token.clone()));
    axum::serve(addr, service)
        .with_graceful_shutdown(shutdown_signal(halt_token))
        .await
        .unwrap_or_else(|e| panic!("{e:?}"));
}

async fn start_loop(cancellation_token: CancellationToken) {
    let worker_loop_result = worker_loop(&cancellation_token).await;
    if let Some(e) = &worker_loop_result.error {
        tracing::error!("worker_loop_shell: serious error occurred in worker loop, halting: {e}");
    }
    tracing::warn!("worker_loop_shell: worker loop halted, trying resuming states...");
    tracing::warn!(
        "worker_loop_shell: friends: {}, pending jobs: {}, last_cursor: {}",
        worker_loop_result.friends.len(),
        worker_loop_result.jobs.len(),
        worker_loop_result.cursor
    );
    resume_state(worker_loop_result);
    cancellation_token.cancel();
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
        tracing::info!("咱其实挺想知道谁会在Windows上跑这个的……");
        tokio::select! {
            _ = signal::ctrl_c() => {},
            _ = cancellation_token.cancelled() => {},
        }
    }
    tracing::error!("cuscuta halting...");
    cancellation_token.cancel();
    halt_progress().await;
}

async fn halt_progress() {
    async fn reset_account_state() -> Result<(), String> {
        let account_row = ACCOUNT_ROW
            .try_read(std::clone::Clone::clone)
            .map_err(|e| format!("failed to fetch account: {e}"))?;
        let transaction = try_open_transaction()
            .await
            .map_err(|e| format!("failed to begin transaction: {e}"))?;
        try_release_account(transaction, account_row.id)
            .await
            .map_err(|e| format!("failed to release account: {e}"))?;
        Ok(())
    }
    if let Err(e) = reset_account_state().await {
        tracing::error!("halt_release: failed to release account: {e}");
    }
}

fn check_ready() -> Option<&'static str> {
    if REDIS_CLIENT.get().is_none() {
        return Some("redis client is not initialized");
    }
    batch_check_initialized!(CONFIG, BUNDLE_DATA, SONG_LIST, ACCOUNT_ROW, POSTGRESQL_POOL);
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
