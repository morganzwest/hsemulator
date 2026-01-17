use crate::engine::run::run;
use crate::{auth::api_key_auth, config::Config, engine::ExecutionMode};

use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
    middleware,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::Span;

/* ---------------- server ---------------- */

pub async fn serve(addr: &str) -> anyhow::Result<()> {
    let protected = Router::new()
        .route("/execute", post(execute))
        .route("/validate", post(validate))
        .layer(middleware::from_fn(api_key_auth));

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
                .on_response(|res: &Response<Body>, latency: Duration, _span: &Span| {
                    tracing::info!(
                        status = res.status().as_u16(),
                        latency_ms = latency.as_millis(),
                        "request completed"
                    );
                }),
        );

    let addr: SocketAddr = addr.parse()?;
    let listener = TcpListener::bind(addr).await?;

    tracing::info!("hsemulate runtime listening on http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

/* ---------------- request models ---------------- */

#[derive(Deserialize)]
struct ExecuteRequest {
    #[serde(default)]
    mode: ExecutionMode,
    config: Config,
}

/* ---------------- endpoints ---------------- */

async fn health() -> &'static str {
    "ok"
}

async fn execute(Json(req): Json<ExecuteRequest>) -> (StatusCode, Json<serde_json::Value>) {
    match run(req.config, req.mode).await {
        Ok(response) => (
            StatusCode::OK,
            Json(serde_json::to_value(response).unwrap()),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": e.to_string(),
            })),
        ),
    }
}

async fn validate(Json(cfg): Json<Config>) -> (StatusCode, Json<serde_json::Value>) {
    match run(cfg, ExecutionMode::Validate).await {
        Ok(response) => (
            StatusCode::OK,
            Json(serde_json::to_value(response).unwrap()),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": e.to_string(),
            })),
        ),
    }
}
