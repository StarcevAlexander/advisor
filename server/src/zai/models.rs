use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: MessageContent,
}

impl ChatMessage {
    pub fn system(text: impl Into<String>) -> Self {
        ChatMessage {
            role: "system".to_string(),
            content: MessageContent::Text(text.into()),
        }
    }

    pub fn user(content: MessageContent) -> Self {
        ChatMessage {
            role: "user".to_string(),
            content,
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        ChatMessage {
            role: "assistant".to_string(),
            content: MessageContent::Text(text.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrlParam },
    #[serde(rename = "file")]
    File { file: FileParam },
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageUrlParam {
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileParam {
    pub file_data: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    #[serde(default)]
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: ChoiceMessage,
    #[serde(default)]
    pub finish_reason: String,
}

#[derive(Debug, Deserialize)]
pub struct ChoiceMessage {
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    #[serde(default)]
    pub message: String,
}
