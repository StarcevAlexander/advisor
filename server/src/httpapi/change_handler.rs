use axum::body::Bytes;
use axum::extract::rejection::BytesRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde_json::json;

use crate::changes::ChangesError;
use crate::store::EVENT_CHANGE_REQUESTED;

use super::constants::{
    MAX_CHANGE_REQUEST_CHARS, MSG_BAD_CHANGE_ID, MSG_BAD_REQUEST, MSG_CHANGE_FAILED,
    MSG_CHANGE_TOO_LONG, MSG_CHANGES_DISABLED, MSG_EMPTY_MESSAGE, MSG_STATUS_FAILED,
};
use super::models::{ChangeRequest, ChangeResponse};
use super::response::{write_error, write_json};
use super::{AppState, CHANGE_ACCEPTED_MESSAGE};

pub async fn handle(
    State(state): State<AppState>,
    body: Result<Bytes, BytesRejection>,
) -> axum::response::Response {
    let Some(requester) = state.changes.clone() else {
        return write_error(StatusCode::NOT_IMPLEMENTED, MSG_CHANGES_DISABLED);
    };
    let bytes = match body {
        Ok(bytes) => bytes,
        Err(_) => return write_error(StatusCode::BAD_REQUEST, MSG_BAD_REQUEST),
    };
    let parsed: ChangeRequest = match serde_json::from_slice(&bytes) {
        Ok(parsed) => parsed,
        Err(_) => return write_error(StatusCode::BAD_REQUEST, MSG_BAD_REQUEST),
    };

    let text = parsed.request.trim().to_string();
    if text.is_empty() {
        return write_error(StatusCode::BAD_REQUEST, MSG_EMPTY_MESSAGE);
    }
    if text.chars().count() > MAX_CHANGE_REQUEST_CHARS {
        return write_error(StatusCode::BAD_REQUEST, MSG_CHANGE_TOO_LONG);
    }

    let result =
        tokio::time::timeout(crate::changes::REQUEST_TIMEOUT, requester.request(&text)).await;
    let receipt = match result {
        Ok(Ok(receipt)) => receipt,
        Ok(Err(err)) => {
            tracing::error!(error = %err, "change request failed");
            return write_error(StatusCode::BAD_GATEWAY, MSG_CHANGE_FAILED);
        }
        Err(_) => {
            tracing::error!("change request failed: request timeout");
            return write_error(StatusCode::BAD_GATEWAY, MSG_CHANGE_FAILED);
        }
    };

    tracing::info!(id = %receipt.id, runes = text.chars().count(), "change requested");
    state.store.save_event(
        EVENT_CHANGE_REQUESTED,
        &receipt.id,
        json!({"request": text, "url": receipt.pulls_url}),
    );
    write_json(
        StatusCode::ACCEPTED,
        &ChangeResponse {
            id: receipt.id,
            message: CHANGE_ACCEPTED_MESSAGE.to_string(),
            url: receipt.pulls_url,
        },
    )
}

pub async fn handle_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> axum::response::Response {
    let Some(requester) = state.changes.clone() else {
        return write_error(StatusCode::NOT_IMPLEMENTED, MSG_CHANGES_DISABLED);
    };
    if !crate::changes::valid_id(&id) {
        return write_error(StatusCode::BAD_REQUEST, MSG_BAD_CHANGE_ID);
    }
    let result = tokio::time::timeout(crate::changes::REQUEST_TIMEOUT, requester.status(&id)).await;
    match result {
        Ok(Ok(status)) => write_json(StatusCode::OK, &status),
        Ok(Err(ChangesError::InvalidID)) => write_error(StatusCode::BAD_REQUEST, MSG_BAD_CHANGE_ID),
        Ok(Err(err)) => {
            tracing::error!(id = %id, error = %err, "change status failed");
            write_error(StatusCode::BAD_GATEWAY, MSG_STATUS_FAILED)
        }
        Err(_) => {
            tracing::error!(id = %id, "change status failed: request timeout");
            write_error(StatusCode::BAD_GATEWAY, MSG_STATUS_FAILED)
        }
    }
}
