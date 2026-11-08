use crate::zai::{ChatMessage, ContentPart, MessageContent};

use super::attachments::{build_content_parts, classify_supported};
use super::constants::{HISTORY_FILES_FORMAT, MAX_HISTORY_TURNS};
use super::models::{ROLE_ASSISTANT, ROLE_USER, Turn};

pub fn history_messages(history: &[Turn], budget: Option<&mut usize>) -> Vec<ChatMessage> {
    let start = history.len().saturating_sub(MAX_HISTORY_TURNS);
    let mut messages: Vec<ChatMessage> = Vec::new();
    let mut budget = budget;
    let include_attachments = budget.is_some();

    for turn in &history[start..] {
        let text = turn_text(turn);
        if turn.role == ROLE_USER {
            let mut parts: Vec<ContentPart> = Vec::new();
            if include_attachments {
                let classified = classify_supported(&turn.attachments);
                let result = budget
                    .as_deref_mut()
                    .map(|b| build_content_parts(&classified, b))
                    .unwrap_or_else(|| Ok(Vec::new()));
                match result {
                    Ok(mut blocks) => parts.append(&mut blocks),
                    Err(err) => {
                        tracing::warn!(error = %err, "history attachments skipped");
                    }
                }
            }
            if !text.is_empty() {
                parts.push(ContentPart::Text {
                    text: text.to_string(),
                });
            }
            if parts.is_empty() {
                continue;
            }
            messages.push(ChatMessage::user(MessageContent::Parts(parts)));
        } else if turn.role == ROLE_ASSISTANT {
            if text.is_empty() || messages.is_empty() {
                continue;
            }
            messages.push(ChatMessage::assistant(text.to_string()));
        }
    }
    messages
}

pub fn build_messages(
    mut messages: Vec<ChatMessage>,
    text: &str,
    attachment_parts: Vec<ContentPart>,
) -> Vec<ChatMessage> {
    let text = text.trim();
    if attachment_parts.is_empty() {
        messages.push(ChatMessage::user(MessageContent::Text(text.to_string())));
        return messages;
    }
    let mut parts = attachment_parts;
    if !text.is_empty() {
        parts.push(ContentPart::Text {
            text: text.to_string(),
        });
    }
    messages.push(ChatMessage::user(MessageContent::Parts(parts)));
    messages
}

pub fn turn_text(turn: &Turn) -> String {
    let text = turn.text.trim();
    if turn.role != ROLE_USER || turn.file_names.is_empty() {
        return text.to_string();
    }
    let marker = HISTORY_FILES_FORMAT.replacen("{}", &turn.file_names.join(", "), 1);
    if text.is_empty() {
        return marker;
    }
    format!("{marker}\n{text}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::models::Attachment;

    const TEST_PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52,
    ];

    fn turn(role: &str, text: &str) -> Turn {
        Turn {
            role: role.to_string(),
            text: text.to_string(),
            ..Turn::default()
        }
    }

    #[test]
    fn drops_leading_assistant_turns() {
        let history = vec![
            turn(ROLE_ASSISTANT, "приветствие"),
            turn("user", "привет"),
            turn(ROLE_ASSISTANT, "здравствуйте"),
        ];
        let messages = history_messages(&history, None);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
    }

    #[test]
    fn limits_history_to_max_turns() {
        let history: Vec<Turn> = (0..30).map(|_| turn("user", "текст")).collect();
        let messages = history_messages(&history, None);
        assert_eq!(messages.len(), MAX_HISTORY_TURNS);
    }

    #[test]
    fn adds_file_names_marker_for_user_turns() {
        let mut t = turn("user", "вопрос");
        t.file_names = vec!["a.pdf".to_string(), "b.png".to_string()];
        assert_eq!(turn_text(&t), "[Приложены файлы: a.pdf, b.png]\nвопрос");
    }

    #[test]
    fn marker_only_when_no_text() {
        let mut t = turn("user", "  ");
        t.file_names = vec!["a.pdf".to_string()];
        assert_eq!(turn_text(&t), "[Приложены файлы: a.pdf]");
    }

    #[test]
    fn history_attachments_become_parts() {
        let mut t = turn("user", "смотри фото");
        t.attachments = vec![Attachment {
            name: "pic.png".to_string(),
            data: TEST_PNG.to_vec(),
        }];
        let messages = history_messages(&[t], Some(&mut (18 << 20)));
        assert_eq!(messages.len(), 1);
        match &messages[0].content {
            MessageContent::Parts(parts) => {
                assert_eq!(parts.len(), 2);
                let value = serde_json::to_value(&parts[0]).unwrap();
                assert_eq!(value["type"], "image_url");
            }
            other => panic!("unexpected content: {other:?}"),
        }
    }

    #[test]
    fn build_messages_appends_user_with_parts_then_text() {
        let messages = build_messages(
            Vec::new(),
            "  вопрос  ",
            vec![ContentPart::Text {
                text: "файл".to_string(),
            }],
        );
        assert_eq!(messages.len(), 1);
        match &messages[0].content {
            MessageContent::Parts(parts) => {
                assert_eq!(parts.len(), 2);
                assert_eq!(serde_json::to_value(&parts[1]).unwrap()["text"], "вопрос");
            }
            other => panic!("unexpected content: {other:?}"),
        }
    }
}
