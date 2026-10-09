//! cuscuta 的 entry。
//!
//! # 依赖环境变量
//! cuscuta-entry 使用环境变量注入参数，这个 crate 依赖的环境变量有：
//! - `REDIS_STREAM_REFRESH_TTL`
//! - `REDIS_ADDR`
//! - `ACCOUNTS_SQL_ADDR`
//! - `STAT_ENABLE`
//! - `RESOURCE_UPDATE_PERIOD`（数据刷新周期，单位秒，默认 30）
//! - `RESOURCE_UPDATE_RETRIES`（数据拉取最大重试次数，默认 5）
//! - `RESOURCES_SONG_URL`

mod config;
mod data;
mod db;
mod endpoints;
mod init;

use crate::{
    config::{Environment, init_env},
    data::SONG_LIST,
    db::{postgresql::POSTGRESQL_POOL, redis::REDIS_CLIENT},
    endpoints::{query::query, status::status},
    enqueue::enqueue,
};
use cuscuta_common::{
    quick_fetch::QuickFetch,
    scheduled_job::{
        register_job_future,
        tasks::{open_postgresql_client, open_redis_client, sync_song_list},
    },
};

use axum::{
    Json, Router,
    http::HeaderValue,
    response::IntoResponse,
    routing::{get, post},
};
use cuscuta_common::{batch_check_initialized, scheduled_job::register_individual_job};
use reqwest::StatusCode;
use serde_json::json;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::{self, TraceLayer},
};
use tracing::Level;

use crate::{endpoints::enqueue, init::cuscuta_init};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    tracing::info!("starting...");
    tracing::info!("reading config...");
    let env = init_env().expect("failed to read env");
    let halt_token = CancellationToken::new();
    tokio::spawn(register_individual_job(
        halt_token.clone(),
        CancellationToken::new(),
        10,
        cuscuta_init,
    ));
    tokio::spawn(register_job_future(
        halt_token.clone(),
        10,
        open_redis_client(&REDIS_CLIENT, env),
    ));
    tokio::spawn(register_job_future(
        halt_token.clone(),
        10,
        open_postgresql_client(&POSTGRESQL_POOL, env),
    ));
    tokio::spawn(register_job_future(
        halt_token.clone(),
        env.resource_update_period,
        sync_song_list(&SONG_LIST, env),
    ));
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
        .on_request(trace::DefaultOnRequest::new().level(Level::INFO))
        .on_response(trace::DefaultOnResponse::new().level(Level::INFO));
    let cors_layer = CorsLayer::new()
        .allow_origin(AllowOrigin::list(parse_cors_origins(env)))
        .allow_credentials(true);
    let service = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/v1/enqueue", post(enqueue).layer(trace_layer.clone()))
        .route("/v1/query", get(query).layer(trace_layer.clone()))
        .route("/v1/status", get(status))
        .layer(cors_layer);
    let addr = TcpListener::bind("0.0.0.0:8081")
        .await
        .expect("failed to bind 0.0.0.0:8081");
    tracing::info!("listening in 0.0.0.0:8081...");
    axum::serve(addr, service)
        .with_graceful_shutdown(shutdown_signal(halt_token))
        .await
        .unwrap_or_else(|e| panic!("{e:?}"));
}

fn parse_cors_origins(env: &Environment) -> Vec<HeaderValue> {
    env.cors_allow_origins
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<HeaderValue>()
                .unwrap_or_else(|e| panic!("CORS origin {s:?} error: {e}"))
        })
        .collect()
}

fn check_ready() -> Option<&'static str> {
    if REDIS_CLIENT.get().is_none() {
        return Some("redis client is not initialized");
    }
    batch_check_initialized!(SONG_LIST, POSTGRESQL_POOL);
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
        tracing::info!("咱其实挺想知道谁会在Windows上跑这个的……");
        tokio::select! {
            _ = signal::ctrl_c() => {},
            _ = cancellation_token.cancelled() => {},
        }
    }
    tracing::error!("cuscuta-entry halting...");
    cancellation_token.cancel();
}
