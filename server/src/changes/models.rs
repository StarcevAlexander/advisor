use serde::{Deserialize, Serialize};

pub const STATE_QUEUED: &str = "queued";
pub const STATE_WORKING: &str = "working";
pub const STATE_READY: &str = "ready";
pub const STATE_DRAFT: &str = "draft";
pub const STATE_MERGED: &str = "merged";
pub const STATE_CLOSED: &str = "closed";
pub const STATE_FAILED: &str = "failed";
pub const STATE_NO_CHANGE: &str = "nochange";

#[derive(Debug, Clone)]
pub struct Receipt {
    pub id: String,
    pub pulls_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pull_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pull_number: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_url: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ChangesError {
    #[error("github rejected the dispatch: {0}")]
    DispatchRejected(String),
    #[error("github lookup failed: {0}")]
    LookupFailed(String),
    #[error("invalid change id")]
    InvalidID,
    #[error("github request: {0}")]
    Request(String),
}

#[async_trait::async_trait]
pub trait Requester: Send + Sync {
    async fn request(&self, text: &str) -> Result<Receipt, ChangesError>;
    async fn status(&self, id: &str) -> Result<Status, ChangesError>;
    async fn rollback(&self) -> Result<Receipt, ChangesError>;
    async fn rollback_status(&self, id: &str) -> Result<Status, ChangesError>;
}

#[derive(Debug, Clone, Default)]
pub struct GitHubConfig {
    pub token: String,
    pub repo: String,
    pub branch: String,
    pub base_url: String,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowRunsResponse {
    #[serde(default)]
    pub workflow_runs: Vec<WorkflowRun>,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowRun {
    #[serde(default)]
    pub display_title: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub conclusion: String,
    #[serde(default)]
    pub html_url: String,
}

#[derive(Debug, Deserialize)]
pub struct PullRequest {
    pub number: i64,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub draft: bool,
    pub merged_at: Option<String>,
}
