use crate::config::Config;
use crate::engine::{execute_action, validate_config};
use crate::execution_id::ExecutionId;
use axum::middleware;
use crate::auth::api_key_auth;

use axum::{
    routing::{get, post},
    Json, Router,
    http::StatusCode,
};
use axum::http::{Request, Response};
use axum::body::Body;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use std::time::Duration;
use tracing::Span;

pub async fn serve(addr: &str) -> anyhow::Result<()> {
    // Protected routes
    let protected = Router::new()
        .route("/execute", post(execute))
        .route("/validate", post(validate))
        .layer(middleware::from_fn(api_key_auth));

    // App router
    let app = Router::new()
    .route("/health", get(health))
    .merge(protected)
    .layer(
        TraceLayer::new_for_http()
            .make_span_with(|req: &Request<Body>| {
                tracing::info_span!(
                    "http_request",
                    method = %req.method(),
                    path = %req.uri().path(),
                )
            })
            .on_response(
                |res: &Response<Body>, latency: Duration, _span: &Span| {
                    tracing::info!(
                        status = res.status().as_u16(),
                        latency_ms = latency.as_millis(),
                        "request completed"
                    );
                },
            ),
    );

    let addr: SocketAddr = addr.parse()?;
    let listener = TcpListener::bind(addr).await?;

    tracing::info!("hsemulate runtime listening on http://{}", addr);

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
