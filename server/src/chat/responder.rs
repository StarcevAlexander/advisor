use crate::zai::{ChatMessage, Client, CompletionRequest};

use super::attachments::{build_content_parts, classify_attachments, default_budget};
use super::constants::{CHAT_REASONING_EFFORT, MAX_OUTPUT_TOKENS, SYSTEM_PROMPT};
use super::messages::{build_messages, history_messages};
use super::models::{ChatError, Conversation, Responder};

pub struct Zai {
    pub(crate) client: Client,
    pub(crate) model: String,
    pub(crate) vision_model: String,
}

impl Zai {
    pub fn new(client: Client, model: String, vision_model: String) -> Self {
        Zai {
            client,
            model,
            vision_model,
        }
    }
}

#[async_trait::async_trait]
impl Responder for Zai {
    async fn reply(&self, conversation: &Conversation) -> Result<String, ChatError> {
        let classified = classify_attachments(&conversation.attachments)?;
        let mut budget = default_budget();
        let parts = build_content_parts(&classified, &mut budget)?;

        let mut messages = history_messages(&conversation.history, Some(&mut budget));
        messages = build_messages(messages, &conversation.text, parts);
        messages.insert(0, ChatMessage::system(SYSTEM_PROMPT));

        let model = if classified.iter().any(|item| item.is_visual()) {
            self.vision_model.clone()
        } else {
            self.model.clone()
        };
        let request = CompletionRequest {
            model,
            messages,
            max_tokens: Some(MAX_OUTPUT_TOKENS),
            reasoning_effort: Some(CHAT_REASONING_EFFORT.to_string()),
        };

        match self.client.complete(&request).await {
            Ok(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    Err(ChatError::EmptyReply)
                } else {
                    Ok(trimmed.to_string())
                }
            }
            Err(crate::zai::Error::Refused) => Ok(super::constants::REFUSAL_REPLY.to_string()),
            Err(err) => Err(ChatError::Zai(err)),
        }
    }
}
