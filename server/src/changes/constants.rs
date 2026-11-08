use std::time::Duration;

pub const WORKFLOW_FILE: &str = "claude-change.yml";
pub const RUN_TITLE_PREFIX: &str = "Claude change ";
pub const BRANCH_PREFIX: &str = "claude/change-";
pub const ROLLBACK_WORKFLOW: &str = "claude-rollback.yml";
pub const ROLLBACK_RUN_PREFIX: &str = "Claude rollback ";
pub const ROLLBACK_BRANCH_HEAD: &str = "claude/rollback-";
pub const DEFAULT_API_BASE_URL: &str = "https://api.github.com";
pub const DEFAULT_REPO: &str = "StarcevAlexander/advisor";
pub const DEFAULT_BRANCH: &str = "master";
pub const API_VERSION: &str = "2022-11-28";
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
pub const MAX_ERROR_BODY_BYTES: usize = 512;
pub const ID_LENGTH: usize = 12;

pub fn valid_id(id: &str) -> bool {
    id.len() == ID_LENGTH && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}
