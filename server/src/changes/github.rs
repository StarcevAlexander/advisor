use serde_json::json;

use super::constants::*;
use super::models::{
    ChangesError, GitHubConfig, PullRequest, Receipt, Requester, STATE_CLOSED, STATE_DRAFT,
    STATE_FAILED, STATE_MERGED, STATE_NO_CHANGE, STATE_QUEUED, STATE_READY, STATE_WORKING, Status,
    WorkflowRun, WorkflowRunsResponse,
};

#[derive(Clone, Copy)]
struct TrackedWorkflow {
    file: &'static str,
    run_prefix: &'static str,
    branch_prefix: &'static str,
}

const CHANGE_WORKFLOW: TrackedWorkflow = TrackedWorkflow {
    file: WORKFLOW_FILE,
    run_prefix: RUN_TITLE_PREFIX,
    branch_prefix: BRANCH_PREFIX,
};

const ROLLBACK_TRACKED_WORKFLOW: TrackedWorkflow = TrackedWorkflow {
    file: ROLLBACK_WORKFLOW,
    run_prefix: ROLLBACK_RUN_PREFIX,
    branch_prefix: ROLLBACK_BRANCH_HEAD,
};

pub struct GitHub {
    config: GitHubConfig,
    http: reqwest::Client,
}

impl GitHub {
    pub fn new(mut config: GitHubConfig) -> Self {
        if config.base_url.trim().is_empty() {
            config.base_url = DEFAULT_API_BASE_URL.to_string();
        }
        if config.repo.trim().is_empty() {
            config.repo = DEFAULT_REPO.to_string();
        }
        if config.branch.trim().is_empty() {
            config.branch = DEFAULT_BRANCH.to_string();
        }
        config.base_url = config.base_url.trim_end_matches('/').to_string();
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|err| ChangesError::Request(err.to_string()))
            .expect("build reqwest client");
        GitHub { config, http }
    }

    async fn dispatch(
        &self,
        workflow: TrackedWorkflow,
        id: &str,
        inputs: serde_json::Value,
    ) -> Result<Receipt, ChangesError> {
        let url = format!(
            "{}/repos/{}/actions/workflows/{}/dispatches",
            self.config.base_url, self.config.repo, workflow.file
        );
        let body = json!({"ref": self.config.branch, "inputs": inputs});
        let response = self
            .http
            .post(&url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .bearer_auth(&self.config.token)
            .json(&body)
            .send()
            .await
            .map_err(|err| ChangesError::Request(err.to_string()))?;

        if response.status().as_u16() != 204 {
            return Err(ChangesError::DispatchRejected(status_error(response).await));
        }
        Ok(Receipt {
            id: id.to_string(),
            pulls_url: format!("https://github.com/{}/pulls", self.config.repo),
        })
    }

    async fn lookup_status(
        &self,
        workflow: TrackedWorkflow,
        id: &str,
    ) -> Result<Status, ChangesError> {
        if !valid_id(id) {
            return Err(ChangesError::InvalidID);
        }

        let runs_url = format!(
            "{}/repos/{}/actions/workflows/{}/runs?event=workflow_dispatch&per_page=30",
            self.config.base_url, self.config.repo, workflow.file
        );
        let runs: WorkflowRunsResponse = self.get_json(&runs_url).await?;
        let run = runs
            .workflow_runs
            .into_iter()
            .find(|item| item.display_title == format!("{}{}", workflow.run_prefix, id));

        let (status, finished_ok) = status_from_run(run);
        if !finished_ok {
            return Ok(status);
        }

        let owner = self.config.repo.split('/').next().unwrap_or_default();
        let pulls_url = format!(
            "{}/repos/{}/pulls?state=all&head={}:{}{}",
            self.config.base_url, self.config.repo, owner, workflow.branch_prefix, id
        );
        let pulls: Vec<PullRequest> = self.get_json(&pulls_url).await?;
        Ok(status_from_pull(status, &pulls))
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, ChangesError> {
        let response = self
            .http
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .bearer_auth(&self.config.token)
            .send()
            .await
            .map_err(|err| ChangesError::LookupFailed(err.to_string()))?;

        if response.status().as_u16() != 200 {
            return Err(ChangesError::LookupFailed(status_error(response).await));
        }
        response
            .json::<T>()
            .await
            .map_err(|err| ChangesError::LookupFailed(format!("decode: {err}")))
    }
}

fn status_from_run(run: Option<WorkflowRun>) -> (Status, bool) {
    let Some(run) = run else {
        return (
            Status {
                state: STATE_QUEUED.to_string(),
                pull_url: None,
                pull_number: None,
                run_url: None,
            },
            false,
        );
    };
    if run.status != "completed" {
        let state = if matches!(run.status.as_str(), "queued" | "waiting" | "pending") {
            STATE_QUEUED
        } else {
            STATE_WORKING
        };
        return (
            Status {
                state: state.to_string(),
                pull_url: None,
                pull_number: None,
                run_url: Some(run.html_url),
            },
            false,
        );
    }
    if run.conclusion != "success" {
        return (
            Status {
                state: STATE_FAILED.to_string(),
                pull_url: None,
                pull_number: None,
                run_url: Some(run.html_url),
            },
            false,
        );
    }
    (
        Status {
            state: STATE_NO_CHANGE.to_string(),
            pull_url: None,
            pull_number: None,
            run_url: Some(run.html_url),
        },
        true,
    )
}

fn status_from_pull(mut base: Status, pulls: &[PullRequest]) -> Status {
    let Some(pull) = pulls.first() else {
        return base;
    };
    base.pull_url = Some(pull.html_url.clone());
    base.pull_number = Some(pull.number);
    base.state = if pull.merged_at.is_some() {
        STATE_MERGED.to_string()
    } else if pull.state == "closed" {
        STATE_CLOSED.to_string()
    } else if pull.draft {
        STATE_DRAFT.to_string()
    } else {
        STATE_READY.to_string()
    };
    base
}

async fn status_error(response: reqwest::Response) -> String {
    let status = response.status().as_u16();
    let body = response.bytes().await.unwrap_or_default();
    let body = &body[..body.len().min(MAX_ERROR_BODY_BYTES)];
    format!("status {status}: {}", String::from_utf8_lossy(body).trim())
}

#[async_trait::async_trait]
impl Requester for GitHub {
    async fn request(&self, text: &str) -> Result<Receipt, ChangesError> {
        let id = crate::util::random_id(6);
        self.dispatch(CHANGE_WORKFLOW, &id, json!({"request": text, "id": id}))
            .await
    }

    async fn status(&self, id: &str) -> Result<Status, ChangesError> {
        self.lookup_status(CHANGE_WORKFLOW, id).await
    }

    async fn rollback(&self) -> Result<Receipt, ChangesError> {
        let id = crate::util::random_id(6);
        self.dispatch(ROLLBACK_TRACKED_WORKFLOW, &id, json!({"id": id}))
            .await
    }

    async fn rollback_status(&self, id: &str) -> Result<Status, ChangesError> {
        self.lookup_status(ROLLBACK_TRACKED_WORKFLOW, id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_id_accepts_lowercase_hex() {
        assert!(valid_id("0a1b2c3d4e5f"));
        assert!(!valid_id("0A1B2C3D4E5F"));
        assert!(!valid_id("short"));
        assert!(!valid_id("0a1b2c3d4e5f6"));
        assert!(!valid_id("zz1b2c3d4e5f"));
    }

    #[test]
    fn status_from_run_covers_all_states() {
        let (status, ok) = status_from_run(None);
        assert_eq!(status.state, STATE_QUEUED);
        assert!(!ok);

        let run = WorkflowRun {
            display_title: String::new(),
            status: "in_progress".to_string(),
            conclusion: String::new(),
            html_url: "run".to_string(),
        };
        let (status, ok) = status_from_run(Some(run));
        assert_eq!(status.state, STATE_WORKING);
        assert!(!ok);

        let run = WorkflowRun {
            status: "completed".to_string(),
            conclusion: "failure".to_string(),
            html_url: "run".to_string(),
            ..run_default()
        };
        let (status, ok) = status_from_run(Some(run));
        assert_eq!(status.state, STATE_FAILED);
        assert!(!ok);

        let run = WorkflowRun {
            status: "completed".to_string(),
            conclusion: "success".to_string(),
            html_url: "run".to_string(),
            ..run_default()
        };
        let (status, ok) = status_from_run(Some(run));
        assert_eq!(status.state, STATE_NO_CHANGE);
        assert!(ok);
    }

    fn run_default() -> WorkflowRun {
        WorkflowRun {
            display_title: String::new(),
            status: String::new(),
            conclusion: String::new(),
            html_url: String::new(),
        }
    }

    #[test]
    fn status_from_pull_orders_states() {
        let base = Status {
            state: STATE_NO_CHANGE.to_string(),
            pull_url: None,
            pull_number: None,
            run_url: None,
        };
        let make_pull = |draft: bool, state: &str, merged_at: Option<String>| PullRequest {
            number: 7,
            html_url: "pr".to_string(),
            state: state.to_string(),
            draft,
            merged_at,
        };

        let pull = make_pull(true, "open", None);
        assert_eq!(status_from_pull(base.clone(), &[pull]).state, STATE_DRAFT);

        let pull = make_pull(false, "open", None);
        let status = status_from_pull(base.clone(), &[pull]);
        assert_eq!(status.state, STATE_READY);
        assert_eq!(status.pull_number, Some(7));

        let pull = make_pull(false, "closed", None);
        assert_eq!(status_from_pull(base.clone(), &[pull]).state, STATE_CLOSED);

        let pull = make_pull(false, "closed", Some("now".to_string()));
        assert_eq!(status_from_pull(base.clone(), &[pull]).state, STATE_MERGED);

        assert_eq!(
            status_from_pull(
                Status {
                    state: STATE_NO_CHANGE.to_string(),
                    pull_url: None,
                    pull_number: None,
                    run_url: None
                },
                &[]
            )
            .state,
            STATE_NO_CHANGE
        );
    }
}
