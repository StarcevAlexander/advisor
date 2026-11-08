use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::chat;

#[derive(Debug, Deserialize)]
pub struct IngestMessage {
    #[serde(rename = "externalId", default)]
    pub external_id: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    #[serde(default)]
    pub source: String,
    #[serde(rename = "conversationId", default)]
    pub conversation_id: String,
    #[serde(default)]
    pub messages: Vec<IngestMessage>,
}

#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub accepted: usize,
}

#[derive(Debug, Deserialize)]
pub struct ClientErrorRequest {
    #[serde(default)]
    pub message: String,
    #[serde(rename = "where", default)]
    pub where_: String,
    #[serde(default)]
    pub stack: String,
    #[serde(default)]
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct ReplyResponse {
    pub reply: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangeRequest {
    #[serde(default)]
    pub request: String,
}

#[derive(Debug, Deserialize)]
pub struct PlanRequest {
    #[serde(default)]
    pub request: String,
    #[serde(default)]
    pub history: Vec<chat::IncomingTurn>,
}

#[derive(Debug, Serialize)]
pub struct ChangeResponse {
    pub id: String,
    pub message: String,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
