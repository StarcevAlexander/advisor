use axum::body::Bytes;
use axum::extract::State;
use axum::extract::rejection::BytesRejection;
use axum::http::StatusCode;
use serde_json::json;

use crate::chat;
use crate::store::EVENT_CHANGE_PLAN;

use super::AppState;
use super::constants::{
    MAX_CHANGE_REQUEST_CHARS, MSG_BAD_REQUEST, MSG_CHANGE_TOO_LONG, MSG_CHANGES_DISABLED,
    MSG_EMPTY_MESSAGE, MSG_PLAN_FAILED,
};
use super::models::PlanRequest;
use super::response::{write_error, write_json};

pub async fn handle(
    State(state): State<AppState>,
    body: Result<Bytes, BytesRejection>,
) -> axum::response::Response {
    if state.changes.is_none() {
        return write_error(StatusCode::NOT_IMPLEMENTED, MSG_CHANGES_DISABLED);
    }
    let bytes = match body {
        Ok(bytes) => bytes,
        Err(_) => return write_error(StatusCode::BAD_REQUEST, MSG_BAD_REQUEST),
    };
    let parsed: PlanRequest = match serde_json::from_slice(&bytes) {
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

    let history: Vec<chat::Turn> = parsed
        .history
        .into_iter()
        .map(|turn| chat::Turn {
            role: turn.role,
            text: turn.text,
            ..chat::Turn::default()
        })
        .collect();

    let result =
        tokio::time::timeout(chat::REQUEST_TIMEOUT, state.planner.plan(&text, &history)).await;
    match result {
        Ok(Ok(plan)) => {
            tracing::info!(steps = plan.steps.len(), "change planned");
            state.store.save_event(
                EVENT_CHANGE_PLAN,
                "",
                json!({"request": text, "plan": plan}),
            );
            write_json(StatusCode::OK, &plan)
        }
        Ok(Err(err)) => {
            tracing::error!(error = %err, "change plan failed");
            write_error(StatusCode::BAD_GATEWAY, MSG_PLAN_FAILED)
        }
        Err(_) => {
            tracing::error!("change plan failed: request timeout");
            write_error(StatusCode::BAD_GATEWAY, MSG_PLAN_FAILED)
        }
    }
}
