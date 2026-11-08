use super::constants::{DETAIL_LIMIT, FINISH_REASON_SENSITIVE, MAX_RESPONSE_BYTES};
use super::models::{ChatResponse, CompletionRequest};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("zai request: {0}")]
    Request(#[source] reqwest::Error),
    #[error("zai read response: {0}")]
    Read(#[source] reqwest::Error),
    #[error("zai http {status}: {detail}")]
    Http {
        status: reqwest::StatusCode,
        detail: String,
    },
    #[error("zai decode response: {0}")]
    Decode(#[source] serde_json::Error),
    #[error("zai error: {0}")]
    Api(String),
    #[error("zai returned no answer")]
    NoAnswer,
    #[error("zai request refused")]
    Refused,
}

pub struct Client {
    base_url: String,
    api_key: String,
    http: reqwest::Client,
}

impl Client {
    pub fn new(api_key: &str, base_url: &str) -> Self {
        Client {
            base_url: base_url.trim().trim_end_matches('/').to_string(),
            api_key: api_key.trim().to_string(),
            http: reqwest::Client::new(),
        }
    }

    pub async fn complete(&self, request: &CompletionRequest) -> Result<String, Error> {
        let url = format!("{}/chat/completions", self.base_url);
        let response = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(request)
            .send()
            .await
            .map_err(Error::Request)?;

        let status = response.status();
        let body = read_limited(response).await?;

        if !status.is_success() {
            return Err(Error::Http {
                status,
                detail: truncate(&body, DETAIL_LIMIT),
            });
        }
        let parsed: ChatResponse = serde_json::from_str(&body).map_err(Error::Decode)?;
        if let Some(error) = &parsed.error
            && !error.message.is_empty()
        {
            return Err(Error::Api(error.message.clone()));
        }
        let Some(choice) = parsed.choices.into_iter().next() else {
            return Err(Error::NoAnswer);
        };
        if choice.finish_reason == FINISH_REASON_SENSITIVE {
            return Err(Error::Refused);
        }
        Ok(choice.message.content.unwrap_or_default())
    }
}

async fn read_limited(response: reqwest::Response) -> Result<String, Error> {
    let mut bytes: Vec<u8> = Vec::new();
    let mut response = response;
    while let Some(chunk) = response.chunk().await.map_err(Error::Read)? {
        if bytes.len() >= MAX_RESPONSE_BYTES {
            break;
        }
        bytes.extend_from_slice(&chunk);
        if bytes.len() > MAX_RESPONSE_BYTES {
            bytes.truncate(MAX_RESPONSE_BYTES);
            break;
        }
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn truncate(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_string();
    }
    let mut end = limit;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zai::models::{ChatMessage, MessageContent};
    use axum::Router;
    use axum::extract::State;
    use axum::routing::post;
    use std::sync::{Arc, Mutex};

    async fn echo_handler(
        State(captured): State<Arc<Mutex<Option<serde_json::Value>>>>,
        body: String,
    ) -> &'static str {
        *captured.lock().unwrap() = serde_json::from_str(&body).ok();
        ""
    }

    #[tokio::test]
    async fn complete_sends_openai_format_and_returns_content() {
        let captured: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
        let state = captured.clone();

        let app = Router::new().route(
            "/chat/completions",
            post(move |req_body: String| {
                let state = state.clone();
                async move { echo_handler(State(state), req_body).await }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let client = Client::new("test-key", &format!("http://{addr}"));
        let request = CompletionRequest {
            model: "glm-5.3".to_string(),
            messages: vec![
                ChatMessage::system("системный промпт"),
                ChatMessage::user(MessageContent::Text("вопрос".to_string())),
            ],
            max_tokens: Some(4096),
            reasoning_effort: Some("low".to_string()),
        };

        let _ = client.complete(&request).await;

        let captured = captured.lock().unwrap().clone().unwrap();
        assert_eq!(captured["model"], "glm-5.3");
        assert_eq!(captured["max_tokens"], 4096);
        assert_eq!(captured["reasoning_effort"], "low");
        let messages = captured["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "системный промпт");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], "вопрос");
    }

    #[tokio::test]
    async fn complete_parses_choices_content() {
        let app = Router::new().route(
            "/chat/completions",
            post(|| async {
                r#"{"choices":[{"message":{"content":"ответ"},"finish_reason":"stop"}]}"#
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let client = Client::new("test-key", &format!("http://{addr}"));
        let request = CompletionRequest {
            model: "m".to_string(),
            messages: vec![ChatMessage::user(MessageContent::Text("q".to_string()))],
            max_tokens: None,
            reasoning_effort: None,
        };
        assert_eq!(client.complete(&request).await.unwrap(), "ответ");
    }

    #[tokio::test]
    async fn complete_maps_sensitive_to_refused() {
        let app = Router::new().route(
            "/chat/completions",
            post(|| async {
                r#"{"choices":[{"message":{"content":""},"finish_reason":"sensitive"}]}"#
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let client = Client::new("k", &format!("http://{addr}"));
        let request = CompletionRequest {
            model: "m".to_string(),
            messages: vec![],
            max_tokens: None,
            reasoning_effort: None,
        };
        assert!(matches!(
            client.complete(&request).await,
            Err(Error::Refused)
        ));
    }

    #[tokio::test]
    async fn complete_maps_api_error_and_empty_choices() {
        let app = Router::new().route(
            "/chat/completions",
            post(|| async { r#"{"error":{"message":"bad key"},"choices":[]}"# }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let client = Client::new("k", &format!("http://{addr}"));
        let request = CompletionRequest {
            model: "m".to_string(),
            messages: vec![],
            max_tokens: None,
            reasoning_effort: None,
        };
        assert!(matches!(client.complete(&request).await, Err(Error::Api(m)) if m == "bad key"));
    }

    #[tokio::test]
    async fn complete_maps_http_error_with_detail() {
        let app = Router::new().route(
            "/chat/completions",
            post(|| async {
                (
                    axum::http::StatusCode::UNAUTHORIZED,
                    r#"{"error":{"message":"invalid api key"}}"#,
                )
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let client = Client::new("k", &format!("http://{addr}"));
        let request = CompletionRequest {
            model: "m".to_string(),
            messages: vec![],
            max_tokens: None,
            reasoning_effort: None,
        };
        match client.complete(&request).await {
            Err(Error::Http { status, detail }) => {
                assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED);
                assert!(detail.contains("invalid api key"));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn truncate_cuts_at_char_boundary() {
        assert_eq!(truncate("hello", 10), "hello");
        let text = "абвгд";
        let cut = truncate(text, 4);
        assert_eq!(cut, "аб");
    }
}
