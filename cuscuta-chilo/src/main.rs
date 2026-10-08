//! cuscuta-chilo是对[chilo](https://github.com/cuscutaceae/chilo)的Web API包装
//!
//! # 依赖环境变量
//! cuscuta-chilo使用环境变量注入参数，这个crate依赖的环境变量有：
//! - `BIN_C1`
//! - `BIN_C2`

mod loop_tasks;

use std::sync::OnceLock;

use axum::{Json, Router, extract::Query, http::StatusCode, response::IntoResponse, routing::get};
use base64::Engine;
use cuscuta_common::{
    batch_check_initialized, data::read_parsed_env, quick_fetch::QuickFetch,
    scheduled_job::register_job,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{net::TcpListener, sync::RwLock};
use tokio_util::sync::CancellationToken;

use crate::loop_tasks::sync_scirpophaga_data;

static C2: OnceLock<RwLock<Option<Vec<u8>>>> = OnceLock::new();

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    tracing::info!("starting...");
    let cancellation_token = CancellationToken::new();
    let data_update_period = read_parsed_env::<u64>("RESOURCE_UPDATE_PERIOD").unwrap_or_else(|e| {
        tracing::info!(
            "pre_init: failed to read RESOURCE_UPDATE_PERIOD: {e}, set to default (30s)"
        );
        30
    });
    tokio::spawn(register_job(
        cancellation_token.clone(),
        data_update_period,
        sync_scirpophaga_data,
    ));
    let service = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/generate", get(generate));
    let addr = TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("failed to bind 0.0.0.0:8080");
    tracing::info!("listening in 0.0.0.0:8080...");
    axum::serve(addr, service)
        .await
        .unwrap_or_else(|e| panic!("{e:?}"));
}

#[derive(Debug, Deserialize)]
struct GenerateQuery {
    timestamp: String,
    body: String,
    path: String,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum GenerateResult {
    Success { success: bool, value: String },
    Failed { success: bool, message: String },
}

async fn generate(Query(form): Query<GenerateQuery>) -> impl IntoResponse {
    tracing::info!("Generated from: {form:?}");
    let Ok(timestamp) = form.timestamp.parse::<u64>() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(GenerateResult::Failed {
                success: false,
                message: "timestamp is not a valid number".into(),
            }),
        );
    };
    let Ok(c2) = C2.read_spinning(std::clone::Clone::clone).await else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(GenerateResult::Failed {
                success: false,
                message: "C2 is not ready".into(),
            }),
        );
    };
    let result = chilo::generate(&c2, timestamp, form.body.as_bytes(), form.path.as_bytes());
    let base64 = base64::prelude::BASE64_STANDARD.encode(result);
    (
        StatusCode::OK,
        Json(GenerateResult::Success {
            success: true,
            value: base64,
        }),
    )
}

fn check_ready() -> Option<&'static str> {
    batch_check_initialized!(C2);
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
