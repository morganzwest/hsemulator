use crate::{
    auth::api_key_auth, config::Config, engine::run::run_execution, engine::ExecutionMode,
};

use crate::engine::events::ExecutionEvent;
use axum::debug_handler;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde::Serialize;
use std::{net::SocketAddr, time::Duration};
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
                .on_response(|res: &Response, latency: Duration, _span: &Span| {
                    tracing::info!(
                        status = res.status().as_u16(),
                        latency_ms = latency.as_millis(),
                        "request completed"
                    );
                }),
        );

    let socket: SocketAddr = addr.parse()?;
    let listener = TcpListener::bind(socket).await?;

    tracing::info!("hsemulate runtime listening on http://{}", socket);

    axum::serve(listener, app).await?;
    Ok(())
}

/* ---------------- request models ---------------- */

#[derive(Debug, Deserialize)]
struct ExecuteRequest {
    #[serde(default)]
    mode: ExecutionMode,
    config: Config,
}

#[derive(Debug, Serialize)]
struct ExecuteResponse {
    summary: crate::engine::summary::ExecutionSummary,
    events: Vec<ExecutionEvent>,
}

/* ---------------- endpoints ---------------- */

async fn health() -> &'static str {
    "ok"
}

#[debug_handler]
async fn execute(Json(req): Json<ExecuteRequest>) -> impl IntoResponse {
    let response: Response = match run_execution(req.config, req.mode).await {
        Ok((summary, sink)) => (
            StatusCode::OK,
            Json(ExecuteResponse {
                summary,
                events: sink.into_events(),
            }),
        )
            .into_response(),

        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": e.to_string(),
            })),
        )
            .into_response(),
    };

    response
}

#[debug_handler]
async fn validate(Json(cfg): Json<Config>) -> impl IntoResponse {
    let response: Response = match run_execution(cfg, ExecutionMode::Validate).await {
        Ok((summary, _sink)) => (StatusCode::OK, Json(summary)).into_response(),

        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": e.to_string(),
            })),
        )
            .into_response(),
    };

    response
}
