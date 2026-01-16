use crate::config::Config;
use crate::engine::{execute_action, validate_config};
use crate::execution_id::ExecutionId;

use axum::{
    routing::{get, post},
    Json, Router,
    http::StatusCode,
};
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub async fn serve(addr: &str) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/execute", post(execute))
        .route("/validate", post(validate));

    let addr: SocketAddr = addr.parse()?;
    let listener = TcpListener::bind(addr).await?;

    eprintln!("hsemulate runtime listening on http://{}", addr);
    eprintln!("health check:    GET  http://{}/health", addr);
    eprintln!("execute action:  POST http://{}/execute", addr);
    eprintln!("validate config: POST http://{}/validate", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

/* ---------------- endpoints ---------------- */

async fn health() -> &'static str {
    "ok"
}

async fn execute(
    Json(cfg): Json<Config>,
) -> (StatusCode, Json<serde_json::Value>) {
    let exec_id = ExecutionId::new();

    match execute_action(cfg, None).await {
        Ok(result) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "execution_id": exec_id,
                "result": result
            })),
        ),

        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "execution_id": exec_id,
                "ok": false,
                "error": e.to_string()
            })),
        ),
    }
}

async fn validate(
    Json(cfg): Json<Config>,
) -> (StatusCode, Json<serde_json::Value>) {
    let exec_id = ExecutionId::new();

    match validate_config(&cfg) {
        Ok(result) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "execution_id": exec_id,
                "validation": result
            })),
        ),

        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "execution_id": exec_id,
                "valid": false,
                "errors": [{
                    "code": "VALIDATION_EXCEPTION",
                    "message": e.to_string()
                }]
            })),
        ),
    }
}
