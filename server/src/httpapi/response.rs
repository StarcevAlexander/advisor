use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;

pub fn write_json<T: Serialize>(status: StatusCode, body: &T) -> Response {
    let mut response = (status, Json(body)).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub fn write_error(status: StatusCode, message: &str) -> Response {
    write_json(
        status,
        &super::ErrorResponse {
            error: message.to_string(),
        },
    )
}
