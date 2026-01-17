use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use std::env;

pub async fn api_key_auth(
    req: Request<Body>,
    next: Next,
) -> Response {
    let expected = match env::var("HSEMULATE_API_KEY") {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "ok": false,
                    "error": "API key not configured"
                })),
            )
                .into_response();
        }
    };

    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    match auth_header {
        Some(value) if value == format!("Bearer {}", expected) => {
            next.run(req).await
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "ok": false,
                "error": "Unauthorized"
            })),
        )
            .into_response(),
    }
}
