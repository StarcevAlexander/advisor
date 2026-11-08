use std::collections::HashMap;

use axum::body::Bytes;
use axum::extract::rejection::BytesRejection;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;

use crate::store::{ERROR_SOURCE_CLIENT, EVENT_INGEST, IncomingMessage, MessageFilter};

use super::AppState;
use super::constants::{
    MAX_CLIENT_ERROR_FIELD_CHARS, MAX_INGEST_MESSAGES, MSG_BAD_INGEST, MSG_BAD_REQUEST,
    MSG_EMPTY_MESSAGE, MSG_HISTORY_FAILED, QUERY_CONVERSATION, QUERY_KIND, QUERY_LIMIT,
    QUERY_SOURCE,
};
use super::models::{ClientErrorRequest, IngestRequest, IngestResponse};
use super::response::{write_error, write_json};

pub async fn handle_history(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> axum::response::Response {
    let filter = MessageFilter {
        source: params.get(QUERY_SOURCE).cloned().unwrap_or_default(),
        conversation_id: params.get(QUERY_CONVERSATION).cloned().unwrap_or_default(),
        limit: limit_param(params.get(QUERY_LIMIT)),
    };
    match state.store.messages(&filter).await {
        Ok(messages) => write_json(StatusCode::OK, &messages),
        Err(err) => {
            tracing::error!(error = %err, "read history failed");
            write_error(StatusCode::INTERNAL_SERVER_ERROR, MSG_HISTORY_FAILED)
        }
    }
}

pub async fn handle_events(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> axum::response::Response {
    let kind = params.get(QUERY_KIND).cloned().unwrap_or_default();
    let limit = limit_param(params.get(QUERY_LIMIT));
    match state.store.events(&kind, limit).await {
        Ok(events) => write_json(StatusCode::OK, &events),
        Err(err) => {
            tracing::error!(error = %err, "read events failed");
            write_error(StatusCode::INTERNAL_SERVER_ERROR, MSG_HISTORY_FAILED)
        }
    }
}

pub async fn handle_errors(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> axum::response::Response {
    let limit = limit_param(params.get(QUERY_LIMIT));
    match state.store.errors(limit).await {
        Ok(entries) => write_json(StatusCode::OK, &entries),
        Err(err) => {
            tracing::error!(error = %err, "read error log failed");
            write_error(StatusCode::INTERNAL_SERVER_ERROR, MSG_HISTORY_FAILED)
        }
    }
}

pub async fn handle_ingest(
    State(state): State<AppState>,
    body: Result<Bytes, BytesRejection>,
) -> axum::response::Response {
    let bytes = match body {
        Ok(bytes) => bytes,
        Err(_) => return write_error(StatusCode::BAD_REQUEST, MSG_BAD_INGEST),
    };
    let parsed: IngestRequest = match serde_json::from_slice(&bytes) {
        Ok(parsed) => parsed,
        Err(_) => return write_error(StatusCode::BAD_REQUEST, MSG_BAD_INGEST),
    };

    let source = parsed.source.trim().to_string();
    if source.is_empty()
        || parsed.messages.is_empty()
        || parsed.messages.len() > MAX_INGEST_MESSAGES
    {
        return write_error(StatusCode::BAD_REQUEST, MSG_BAD_INGEST);
    }

    let mut messages: Vec<IncomingMessage> = Vec::with_capacity(parsed.messages.len());
    for message in parsed.messages {
        let text = message.text.trim().to_string();
        if text.is_empty() || message.role.is_empty() {
            continue;
        }
        messages.push(IncomingMessage {
            source: source.clone(),
            conversation_id: parsed.conversation_id.clone(),
            external_id: message.external_id,
            role: message.role,
            text,
            attachments_names: Vec::new(),
            meta: None,
            created_at: message.at,
        });
    }

    let accepted = messages.len();
    state.store.save_messages(messages);
    state.store.save_event(
        EVENT_INGEST,
        &parsed.conversation_id,
        json!({"source": source, "messages": accepted}),
    );
    write_json(StatusCode::ACCEPTED, &IngestResponse { accepted })
}

pub async fn handle_client_error(
    State(state): State<AppState>,
    body: Result<Bytes, BytesRejection>,
) -> axum::response::Response {
    let bytes = match body {
        Ok(bytes) => bytes,
        Err(_) => return write_error(StatusCode::BAD_REQUEST, MSG_BAD_REQUEST),
    };
    let parsed: ClientErrorRequest = match serde_json::from_slice(&bytes) {
        Ok(parsed) => parsed,
        Err(_) => return write_error(StatusCode::BAD_REQUEST, MSG_BAD_REQUEST),
    };

    let message = clip_runes(parsed.message.trim(), MAX_CLIENT_ERROR_FIELD_CHARS);
    if message.is_empty() {
        return write_error(StatusCode::BAD_REQUEST, MSG_EMPTY_MESSAGE);
    }
    state.store.save_error(
        ERROR_SOURCE_CLIENT,
        &message,
        json!({
            "where": clip_runes(&parsed.where_, MAX_CLIENT_ERROR_FIELD_CHARS),
            "stack": clip_runes(&parsed.stack, MAX_CLIENT_ERROR_FIELD_CHARS * 4),
            "version": clip_runes(&parsed.version, 64),
        }),
    );
    StatusCode::NO_CONTENT.into_response()
}

fn limit_param(raw: Option<&String>) -> i64 {
    raw.and_then(|value| value.trim().parse().ok()).unwrap_or(0)
}

fn clip_runes(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    text.chars().take(max_chars).collect()
}
