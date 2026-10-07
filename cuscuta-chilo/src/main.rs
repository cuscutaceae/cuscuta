//! cuscuta-chilo是对[chilo](https://github.com/cuscutaceae/chilo)的Web API包装
//!
//! # 依赖环境变量
//! cuscuta-chilo使用环境变量注入参数，这个crate依赖的环境变量有：
//! - `BIN_C1`
//! - `BIN_C2`

#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]

use std::{env, sync::OnceLock};

use axum::{Json, Router, extract::Query, http::StatusCode, response::IntoResponse, routing::get};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::TcpListener;

static C2: OnceLock<Vec<u8>> = OnceLock::new();

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    tracing::info!("starting...");
    C2.set(read_hex("BIN_C2")).expect("failed to set C2");
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

fn read_hex(var: &str) -> Vec<u8> {
    hex::decode(env::var(var).unwrap_or_else(|_| panic!("var: {var} not declared")))
        .unwrap_or_else(|_| panic!("failed to decode {var}"))
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
    let result = chilo::generate(
        C2.get()
            .expect("C2 is not initialized, this should not happen"),
        timestamp,
        form.body.as_bytes(),
        form.path.as_bytes(),
    );
    let base64 = base64::prelude::BASE64_STANDARD.encode(result);
    (
        StatusCode::OK,
        Json(GenerateResult::Success {
            success: true,
            value: base64,
        }),
    )
}

const fn check_ready() -> Option<String> {
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
