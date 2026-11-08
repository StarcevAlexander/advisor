use std::collections::HashMap;

use axum::extract::multipart::MultipartRejection;
use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use chrono::Utc;
use serde_json::json;

use crate::chat;
use crate::store::{
    self, EVENT_CHAT_REQUEST, IncomingMessage, MessageFilter, ROLE_ASSISTANT, ROLE_USER,
    SOURCE_APP, StoredFile, TurnMeta,
};

use super::AppState;
use super::constants::{
    CONTEXT_ATTACHMENT_TURNS, FIELD_CONVERSATION, FIELD_FILES, FIELD_HISTORY, FIELD_TEXT,
    HISTORY_ATTACHMENT_MAX_BYTES, HISTORY_SOURCE_CLIENT, HISTORY_SOURCE_SERVER, MAX_FILES,
    MSG_BAD_HISTORY, MSG_BAD_REQUEST, MSG_TOO_LARGE, MSG_TOO_MANY_FILES, MSG_UNSUPPORTED_FILE,
    MSG_UPSTREAM_FAILED,
};
use super::models::ReplyResponse;
use super::response::{write_error, write_json};

pub async fn handle(
    State(state): State<AppState>,
    multipart: Result<Multipart, MultipartRejection>,
) -> axum::response::Response {
    let mut multipart = match multipart {
        Ok(multipart) => multipart,
        Err(rejection) => return multipart_rejection_response(rejection),
    };

    let mut text = String::new();
    let mut history_raw = String::new();
    let mut conversation_id = String::new();
    let mut attachments: Vec<chat::Attachment> = Vec::new();

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(err) => {
                if err.status() == StatusCode::PAYLOAD_TOO_LARGE {
                    return write_error(StatusCode::PAYLOAD_TOO_LARGE, MSG_TOO_LARGE);
                }
                return write_error(StatusCode::BAD_REQUEST, MSG_BAD_REQUEST);
            }
        };
        match field.name().unwrap_or_default() {
            FIELD_TEXT => text = field_text(field).await,
            FIELD_HISTORY => history_raw = field_text(field).await,
            FIELD_CONVERSATION => conversation_id = field_text(field).await,
            FIELD_FILES => {
                if attachments.len() >= MAX_FILES {
                    return write_error(StatusCode::BAD_REQUEST, MSG_TOO_MANY_FILES);
                }
                let name = field.file_name().unwrap_or_default().to_string();
                let data = match field.bytes().await {
                    Ok(bytes) => bytes.to_vec(),
                    Err(err) => {
                        if err.status() == StatusCode::PAYLOAD_TOO_LARGE {
                            return write_error(StatusCode::PAYLOAD_TOO_LARGE, MSG_TOO_LARGE);
                        }
                        return write_error(StatusCode::BAD_REQUEST, MSG_BAD_REQUEST);
                    }
                };
                attachments.push(chat::Attachment { name, data });
            }
            _ => {
                let _ = field.bytes().await;
            }
        }
    }

    let mut history: Vec<chat::Turn> = Vec::new();
    if !history_raw.is_empty() {
        match serde_json::from_str::<Vec<chat::IncomingTurn>>(&history_raw) {
            Ok(turns) => {
                history = turns
                    .into_iter()
                    .map(|turn| chat::Turn {
                        role: turn.role,
                        text: turn.text,
                        ..chat::Turn::default()
                    })
                    .collect();
            }
            Err(_) => return write_error(StatusCode::BAD_REQUEST, MSG_BAD_HISTORY),
        }
    }

    let text = text.trim().to_string();
    if text.is_empty() && attachments.is_empty() {
        return write_error(StatusCode::BAD_REQUEST, super::constants::MSG_EMPTY_MESSAGE);
    }

    let mut conversation = chat::Conversation {
        history,
        text,
        attachments,
    };

    let mut history_source = HISTORY_SOURCE_CLIENT;
    if !conversation_id.is_empty()
        && let Some(turns) = stored_history(&state, &conversation_id).await
    {
        conversation.history = turns;
        history_source = HISTORY_SOURCE_SERVER;
    }

    let reply =
        tokio::time::timeout(chat::REQUEST_TIMEOUT, state.responder.reply(&conversation)).await;

    match reply {
        Ok(Ok(text)) => {
            record(
                &state,
                &conversation_id,
                history_source,
                &conversation,
                &text,
            );
            write_json(StatusCode::OK, &ReplyResponse { reply: text })
        }
        Ok(Err(chat::ChatError::UnsupportedAttachment)) => {
            write_error(StatusCode::UNSUPPORTED_MEDIA_TYPE, MSG_UNSUPPORTED_FILE)
        }
        Ok(Err(chat::ChatError::AttachmentTooLarge)) => {
            write_error(StatusCode::PAYLOAD_TOO_LARGE, MSG_TOO_LARGE)
        }
        Ok(Err(err)) => {
            tracing::error!(error = %err, "reply failed");
            write_error(StatusCode::BAD_GATEWAY, MSG_UPSTREAM_FAILED)
        }
        Err(_) => {
            tracing::error!("reply failed: request timeout");
            write_error(StatusCode::BAD_GATEWAY, MSG_UPSTREAM_FAILED)
        }
    }
}

async fn field_text(field: axum::extract::multipart::Field<'_>) -> String {
    match field.bytes().await {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(_) => String::new(),
    }
}

fn multipart_rejection_response(rejection: MultipartRejection) -> axum::response::Response {
    let response = axum::response::IntoResponse::into_response(rejection);
    if response.status() == StatusCode::PAYLOAD_TOO_LARGE {
        write_error(StatusCode::PAYLOAD_TOO_LARGE, MSG_TOO_LARGE)
    } else {
        write_error(StatusCode::BAD_REQUEST, MSG_BAD_REQUEST)
    }
}

async fn stored_history(state: &AppState, conversation_id: &str) -> Option<Vec<chat::Turn>> {
    let messages = state
        .store
        .messages(&MessageFilter {
            source: SOURCE_APP.to_string(),
            conversation_id: conversation_id.to_string(),
            limit: chat::MAX_HISTORY_TURNS as i64,
        })
        .await;
    let messages = match messages {
        Ok(messages) => messages,
        Err(err) => {
            tracing::warn!(error = %err, "stored history unavailable, using client history");
            return None;
        }
    };
    if messages.is_empty() {
        return None;
    }

    let mut turns: Vec<chat::Turn> = Vec::with_capacity(messages.len());
    let mut turn_index: HashMap<String, usize> = HashMap::new();
    let mut with_files: Vec<String> = Vec::new();
    for message in &messages {
        let mut turn = chat::Turn {
            role: message.role.clone(),
            text: message.text.clone(),
            ..chat::Turn::default()
        };
        if message.role == ROLE_USER && !message.attachments.is_empty() {
            turn.file_names = message.attachments.clone();
            let turn_id = store::message_turn_id(&message.meta);
            if !turn_id.is_empty() {
                turn_index.insert(turn_id.clone(), turns.len());
                with_files.push(turn_id);
            }
        }
        turns.push(turn);
    }

    if with_files.len() > CONTEXT_ATTACHMENT_TURNS {
        let skip = with_files.len() - CONTEXT_ATTACHMENT_TURNS;
        with_files.drain(..skip);
    }

    match state
        .store
        .attachments(conversation_id, &with_files, HISTORY_ATTACHMENT_MAX_BYTES)
        .await
    {
        Ok(files) => {
            for file in files {
                if let Some(&index) = turn_index.get(&file.turn_id) {
                    turns[index].attachments.push(chat::Attachment {
                        name: file.name,
                        data: file.data,
                    });
                }
            }
        }
        Err(err) => {
            tracing::warn!(error = %err, "stored attachments unavailable");
        }
    }
    Some(turns)
}

fn record(
    state: &AppState,
    conversation_id: &str,
    history_source: &str,
    conversation: &chat::Conversation,
    reply: &str,
) {
    let names: Vec<String> = conversation
        .attachments
        .iter()
        .map(|attachment| attachment.name.clone())
        .collect();
    let files: Vec<StoredFile> = conversation
        .attachments
        .iter()
        .map(|attachment| StoredFile {
            turn_id: String::new(),
            name: attachment.name.clone(),
            data: attachment.data.clone(),
        })
        .collect();

    let turn_id = crate::util::random_id(16);
    let meta = serde_json::to_value(TurnMeta {
        turn: turn_id.clone(),
    })
    .ok();
    let now = Utc::now();

    state
        .store
        .save_attachments(conversation_id, &turn_id, files);
    state.store.save_messages(vec![
        IncomingMessage {
            source: SOURCE_APP.to_string(),
            conversation_id: conversation_id.to_string(),
            external_id: String::new(),
            role: ROLE_USER.to_string(),
            text: conversation.text.clone(),
            attachments_names: names.clone(),
            meta: meta.clone(),
            created_at: Some(now),
        },
        IncomingMessage {
            source: SOURCE_APP.to_string(),
            conversation_id: conversation_id.to_string(),
            external_id: String::new(),
            role: ROLE_ASSISTANT.to_string(),
            text: reply.to_string(),
            attachments_names: Vec::new(),
            meta: None,
            created_at: Some(now + chrono::Duration::milliseconds(1)),
        },
    ]);
    state.store.save_event(
        EVENT_CHAT_REQUEST,
        conversation_id,
        json!({
            "historyTurns": conversation.history.len(),
            "historySource": history_source,
            "attachments": names.len(),
        }),
    );
}
