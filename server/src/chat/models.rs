use serde::{Deserialize, Serialize};

pub const ROLE_USER: &str = "user";
pub const ROLE_ASSISTANT: &str = "assistant";

#[derive(Debug, Clone, Deserialize)]
pub struct IncomingTurn {
    pub role: String,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct Turn {
    pub role: String,
    pub text: String,
    pub file_names: Vec<String>,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone)]
pub struct Attachment {
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct Conversation {
    pub history: Vec<Turn>,
    pub text: String,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub summary: String,
    #[serde(default)]
    pub steps: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    #[error("unsupported attachment")]
    UnsupportedAttachment,
    #[error("attachment too large")]
    AttachmentTooLarge,
    #[error("empty reply")]
    EmptyReply,
    #[error("bad plan")]
    BadPlan,
    #[error(transparent)]
    Zai(#[from] crate::zai::Error),
}

#[async_trait::async_trait]
pub trait Responder: Send + Sync {
    async fn reply(&self, conversation: &Conversation) -> Result<String, ChatError>;
}

#[async_trait::async_trait]
pub trait Planner: Send + Sync {
    async fn plan(&self, request: &str, history: &[Turn]) -> Result<Plan, ChatError>;
}
