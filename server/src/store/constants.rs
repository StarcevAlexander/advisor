use std::time::Duration;

pub const SOURCE_APP: &str = "app";

pub const ROLE_USER: &str = "user";
pub const ROLE_ASSISTANT: &str = "assistant";

pub const EVENT_CHAT_REQUEST: &str = "chat.request";
pub const EVENT_CHANGE_PLAN: &str = "change.plan";
pub const EVENT_CHANGE_REQUESTED: &str = "change.requested";
pub const EVENT_ROLLBACK_REQUEST: &str = "rollback.requested";
pub const EVENT_INGEST: &str = "ingest";
pub const ERROR_SOURCE_SERVER: &str = "server";
pub const ERROR_SOURCE_CLIENT: &str = "client";

pub const QUEUE_SIZE: usize = 512;
pub const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
pub const READ_TIMEOUT: Duration = Duration::from_secs(10);
pub const CLOSE_TIMEOUT: Duration = Duration::from_secs(8);
pub const DEFAULT_LIST_LIMIT: i64 = 100;
pub const MAX_LIST_LIMIT: i64 = 1000;
pub const MAX_STORED_TEXT_BYTES: usize = 512 << 10;

pub fn clamp_limit(limit: i64) -> i64 {
    match limit {
        ..=0 => DEFAULT_LIST_LIMIT,
        1..=MAX_LIST_LIMIT => limit,
        _ => MAX_LIST_LIMIT,
    }
}

pub fn clip(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_limit_bounds() {
        assert_eq!(clamp_limit(0), DEFAULT_LIST_LIMIT);
        assert_eq!(clamp_limit(-5), DEFAULT_LIST_LIMIT);
        assert_eq!(clamp_limit(25), 25);
        assert_eq!(clamp_limit(MAX_LIST_LIMIT), MAX_LIST_LIMIT);
        assert_eq!(clamp_limit(MAX_LIST_LIMIT + 1), MAX_LIST_LIMIT);
    }

    #[test]
    fn clip_truncates_at_char_boundary() {
        assert_eq!(clip("short", 10), "short");
        let text = "абвгд";
        assert_eq!(clip(text, 4), "аб");
        assert_eq!(clip(text, 100), text);
    }
}
