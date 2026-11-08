use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub id: i64,
    pub source: String,
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "externalId", skip_serializing_if = "String::is_empty")]
    pub external_id: String,
    pub role: String,
    pub text: String,
    pub attachments: Vec<String>,
    #[serde(skip_serializing_if = "Value::is_null")]
    pub meta: Value,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub id: i64,
    pub kind: String,
    pub reference: String,
    pub payload: Value,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorEntry {
    pub id: i64,
    pub source: String,
    pub message: String,
    #[serde(skip_serializing_if = "Value::is_null")]
    pub details: Value,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnMeta {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub turn: String,
}

#[derive(Debug, Clone)]
pub struct StoredFile {
    pub turn_id: String,
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct MessageFilter {
    pub source: String,
    pub conversation_id: String,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct IncomingMessage {
    pub source: String,
    pub conversation_id: String,
    pub external_id: String,
    pub role: String,
    pub text: String,
    pub attachments_names: Vec<String>,
    pub meta: Option<Value>,
    pub created_at: Option<DateTime<Utc>>,
}
