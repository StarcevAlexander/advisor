use std::time::Instant;

use axum::extract::{Request, State};
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use super::AppState;
use super::constants::{AUTH_FAIL_DELAY, BEARER_PREFIX, HEALTH_PATH, MSG_UNAUTHORIZED};
use super::response::write_error;

pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let provided_hash: [u8; 32] = Sha256::digest(bearer_token(&request).as_bytes()).into();
    if provided_hash.ct_eq(&state.password_hash).unwrap_u8() != 1 {
        tokio::time::sleep(AUTH_FAIL_DELAY).await;
        return write_error(StatusCode::UNAUTHORIZED, MSG_UNAUTHORIZED);
    }
    next.run(request).await
}

fn bearer_token(request: &Request) -> String {
    request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix(BEARER_PREFIX))
        .unwrap_or_default()
        .to_string()
}

pub async fn log_middleware(request: Request, next: Next) -> Response {
    if request.uri().path() == HEALTH_PATH || request.method() == Method::OPTIONS {
        return next.run(request).await;
    }
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let bytes = request
        .headers()
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(-1);
    let started = Instant::now();
    let response = next.run(request).await;
    let took_ms = started.elapsed().as_millis() as i64;
    tracing::info!(
        method = %method,
        path = %path,
        status = response.status().as_u16(),
        bytes = bytes,
        took_ms = took_ms,
        "request"
    );
    response
}
