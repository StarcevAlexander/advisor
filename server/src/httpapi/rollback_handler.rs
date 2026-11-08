use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde_json::json;

use crate::changes::ChangesError;
use crate::store::EVENT_ROLLBACK_REQUEST;

use super::constants::{
    MSG_BAD_CHANGE_ID, MSG_CHANGES_DISABLED, MSG_ROLLBACK_FAILED, MSG_STATUS_FAILED,
};
use super::models::ChangeResponse;
use super::response::{write_error, write_json};
use super::{AppState, ROLLBACK_ACCEPTED_MESSAGE};

pub async fn handle(State(state): State<AppState>) -> axum::response::Response {
    let Some(requester) = state.changes.clone() else {
        return write_error(StatusCode::NOT_IMPLEMENTED, MSG_CHANGES_DISABLED);
    };
    let result = tokio::time::timeout(crate::changes::REQUEST_TIMEOUT, requester.rollback()).await;
    let receipt = match result {
        Ok(Ok(receipt)) => receipt,
        Ok(Err(err)) => {
            tracing::error!(error = %err, "rollback request failed");
            return write_error(StatusCode::BAD_GATEWAY, MSG_ROLLBACK_FAILED);
        }
        Err(_) => {
            tracing::error!("rollback request failed: request timeout");
            return write_error(StatusCode::BAD_GATEWAY, MSG_ROLLBACK_FAILED);
        }
    };

    tracing::info!(id = %receipt.id, "rollback requested");
    state.store.save_event(
        EVENT_ROLLBACK_REQUEST,
        &receipt.id,
        json!({"url": receipt.pulls_url}),
    );
    write_json(
        StatusCode::ACCEPTED,
        &ChangeResponse {
            id: receipt.id,
            message: ROLLBACK_ACCEPTED_MESSAGE.to_string(),
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
    let result = tokio::time::timeout(
        crate::changes::REQUEST_TIMEOUT,
        requester.rollback_status(&id),
    )
    .await;
    match result {
        Ok(Ok(status)) => write_json(StatusCode::OK, &status),
        Ok(Err(ChangesError::InvalidID)) => write_error(StatusCode::BAD_REQUEST, MSG_BAD_CHANGE_ID),
        Ok(Err(err)) => {
            tracing::error!(id = %id, error = %err, "rollback status failed");
            write_error(StatusCode::BAD_GATEWAY, MSG_STATUS_FAILED)
        }
        Err(_) => {
            tracing::error!(id = %id, "rollback status failed: request timeout");
            write_error(StatusCode::BAD_GATEWAY, MSG_STATUS_FAILED)
        }
    }
}
