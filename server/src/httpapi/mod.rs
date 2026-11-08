mod change_handler;
mod chat_handler;
mod constants;
mod history_handler;
mod middleware;
mod models;
mod plan_handler;
mod response;
mod rollback_handler;

pub use constants::*;
pub use models::*;

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::middleware::from_fn_with_state;
use axum::routing::{get, post};
use sha2::{Digest, Sha256};
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::changes;
use crate::chat;
use crate::store::Store;

use self::middleware::{auth_middleware, log_middleware};

#[derive(Clone)]
pub struct AppState {
    pub responder: Arc<dyn chat::Responder>,
    pub planner: Arc<dyn chat::Planner>,
    pub changes: Option<Arc<dyn changes::Requester>>,
    pub password_hash: [u8; 32],
    pub store: Arc<dyn Store>,
}

pub struct Options {
    pub responder: Arc<dyn chat::Responder>,
    pub planner: Arc<dyn chat::Planner>,
    pub changes: Option<Arc<dyn changes::Requester>>,
    pub password: String,
    pub allowed_origins: Vec<String>,
    pub store: Arc<dyn Store>,
}

pub fn router(options: Options) -> Router {
    let state = AppState {
        responder: options.responder,
        planner: options.planner,
        changes: options.changes,
        password_hash: Sha256::digest(options.password.as_bytes()).into(),
        store: options.store,
    };

    let api = Router::new()
        .route(
            "/chat",
            post(chat_handler::handle).layer(DefaultBodyLimit::max(MAX_BODY_BYTES)),
        )
        .route(
            "/change",
            post(change_handler::handle).layer(DefaultBodyLimit::max(MAX_CHANGE_BODY_BYTES)),
        )
        .route(
            "/change/plan",
            post(plan_handler::handle).layer(DefaultBodyLimit::max(MAX_PLAN_BODY_BYTES)),
        )
        .route("/change/{id}", get(change_handler::handle_status))
        .route("/rollback", post(rollback_handler::handle))
        .route("/rollback/{id}", get(rollback_handler::handle_status))
        .route("/history", get(history_handler::handle_history))
        .route("/events", get(history_handler::handle_events))
        .route("/errors", get(history_handler::handle_errors))
        .route(
            "/ingest",
            post(history_handler::handle_ingest)
                .layer(DefaultBodyLimit::max(MAX_INGEST_BODY_BYTES)),
        )
        .route(
            "/client-error",
            post(history_handler::handle_client_error)
                .layer(DefaultBodyLimit::max(MAX_CLIENT_ERROR_BYTES)),
        )
        .layer(from_fn_with_state(state.clone(), auth_middleware));

    Router::new()
        .nest("/api", api)
        .route("/healthz", get(|| async { StatusCode::NO_CONTENT }))
        .layer(CatchPanicLayer::custom(|_| {
            response::write_error(StatusCode::INTERNAL_SERVER_ERROR, MSG_UPSTREAM_FAILED)
        }))
        .layer(axum::middleware::from_fn(log_middleware))
        .layer(cors_layer(options.allowed_origins))
        .with_state(state)
}

fn cors_layer(allowed_origins: Vec<String>) -> CorsLayer {
    let origins: Vec<HeaderValue> = allowed_origins
        .iter()
        .filter_map(|origin| origin.parse().ok())
        .collect();
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .max_age(Duration::from_secs(600))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changes::{ChangesError, Receipt, Requester, Status};
    use crate::chat::{ChatError, Conversation, Plan, Planner, Responder, Turn};
    use crate::store::Noop;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::sync::{Arc, Mutex};
    use tower::util::ServiceExt;

    const PASSWORD: &str = "test-password";
    const BOUNDARY: &str = "testboundary42";
    const TEST_PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52,
    ];

    #[derive(Clone)]
    enum ReplyOutcome {
        Text(String),
        Unsupported,
        TooLarge,
        NoAnswer,
    }

    struct FakeResponder {
        reply: Mutex<ReplyOutcome>,
        conversations: Mutex<Vec<Conversation>>,
    }

    #[async_trait::async_trait]
    impl Responder for FakeResponder {
        async fn reply(&self, conversation: &Conversation) -> Result<String, ChatError> {
            self.conversations
                .lock()
                .unwrap()
                .push(conversation.clone());
            match self.reply.lock().unwrap().clone() {
                ReplyOutcome::Text(text) => Ok(text),
                ReplyOutcome::Unsupported => Err(ChatError::UnsupportedAttachment),
                ReplyOutcome::TooLarge => Err(ChatError::AttachmentTooLarge),
                ReplyOutcome::NoAnswer => Err(ChatError::Zai(crate::zai::Error::NoAnswer)),
            }
        }
    }

    struct FakePlanner;

    #[async_trait::async_trait]
    impl Planner for FakePlanner {
        async fn plan(&self, _request: &str, _history: &[Turn]) -> Result<Plan, ChatError> {
            Ok(Plan {
                summary: "понял задачу".to_string(),
                steps: vec!["шаг один".to_string()],
            })
        }
    }

    struct FakeRequester;

    #[async_trait::async_trait]
    impl Requester for FakeRequester {
        async fn request(&self, _text: &str) -> Result<Receipt, ChangesError> {
            Ok(Receipt {
                id: "0a1b2c3d4e5f".to_string(),
                pulls_url: "https://github.com/test/pulls".to_string(),
            })
        }
        async fn status(&self, _id: &str) -> Result<Status, ChangesError> {
            Ok(Status {
                state: "queued".to_string(),
                pull_url: None,
                pull_number: None,
                run_url: None,
            })
        }
        async fn rollback(&self) -> Result<Receipt, ChangesError> {
            self.request("").await
        }
        async fn rollback_status(&self, id: &str) -> Result<Status, ChangesError> {
            self.status(id).await
        }
    }

    fn test_router(responder: Arc<FakeResponder>, changes: Option<Arc<dyn Requester>>) -> Router {
        router(Options {
            responder,
            planner: Arc::new(FakePlanner),
            changes,
            password: PASSWORD.to_string(),
            allowed_origins: vec!["http://localhost:4200".to_string()],
            store: Arc::new(Noop),
        })
    }

    fn ok_responder() -> Arc<FakeResponder> {
        Arc::new(FakeResponder {
            reply: Mutex::new(ReplyOutcome::Text("ответ советчика".to_string())),
            conversations: Mutex::new(Vec::new()),
        })
    }

    fn err_responder(outcome: ReplyOutcome) -> Arc<FakeResponder> {
        Arc::new(FakeResponder {
            reply: Mutex::new(outcome),
            conversations: Mutex::new(Vec::new()),
        })
    }

    fn multipart_part(name: &str, filename: Option<&str>, data: &[u8]) -> Vec<u8> {
        let mut part =
            format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"").into_bytes();
        if let Some(filename) = filename {
            part.extend_from_slice(format!("; filename=\"{filename}\"").as_bytes());
        }
        part.extend_from_slice(b"\r\nContent-Type: application/octet-stream\r\n\r\n");
        part.extend_from_slice(data);
        part.extend_from_slice(b"\r\n");
        part
    }

    fn multipart_body(parts: Vec<Vec<u8>>) -> Vec<u8> {
        let mut body = parts.concat();
        body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
        body
    }

    fn post_chat(authorization: Option<&str>, body: Vec<u8>) -> Request<Body> {
        let mut builder = Request::builder().method("POST").uri("/api/chat").header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        );
        if let Some(authorization) = authorization {
            builder = builder.header("authorization", authorization);
        }
        builder.body(Body::from(body)).unwrap()
    }

    fn post_json(uri: &str, authorization: Option<&str>, body: &str) -> Request<Body> {
        let mut builder = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json");
        if let Some(authorization) = authorization {
            builder = builder.header("authorization", authorization);
        }
        builder.body(Body::from(body.to_string())).unwrap()
    }

    fn get(uri: &str, authorization: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder().uri(uri);
        if let Some(authorization) = authorization {
            builder = builder.header("authorization", authorization);
        }
        builder.body(Body::empty()).unwrap()
    }

    async fn body_text(response: axum::response::Response) -> String {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    #[tokio::test]
    async fn healthz_open_without_password() {
        let response = test_router(ok_responder(), None)
            .oneshot(get("/healthz", None))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn chat_rejects_missing_password() {
        let response = test_router(ok_responder(), None)
            .oneshot(post_chat(
                None,
                multipart_body(vec![multipart_part("text", None, "привет".as_bytes())]),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = body_text(response).await;
        assert!(body.contains(MSG_UNAUTHORIZED));
    }

    #[tokio::test]
    async fn chat_rejects_wrong_password() {
        let response = test_router(ok_responder(), None)
            .oneshot(post_chat(
                Some("Bearer wrong-password"),
                multipart_body(vec![multipart_part("text", None, "привет".as_bytes())]),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn chat_answers_text_message() {
        let responder = ok_responder();
        let router = test_router(responder.clone(), None);
        let body = multipart_body(vec![
            multipart_part("text", None, "  какой вариант выбрать?  ".as_bytes()),
            multipart_part(
                "history",
                None,
                r#"[{"role":"user","text":"привет"},{"role":"assistant","text":"здравствуйте"}]"#
                    .as_bytes(),
            ),
            multipart_part("conversation", None, b"conv-1"),
        ]);
        let response = router
            .oneshot(post_chat(Some(&format!("Bearer {PASSWORD}")), body))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let text = body_text(response).await;
        assert!(text.contains("ответ советчика"));

        let conversations = responder.conversations.lock().unwrap();
        let conversation = conversations.last().unwrap();
        assert_eq!(conversation.text, "какой вариант выбрать?");
        assert_eq!(conversation.history.len(), 2);
    }

    #[tokio::test]
    async fn chat_requires_text_or_files() {
        let response = test_router(ok_responder(), None)
            .oneshot(post_chat(
                Some(&format!("Bearer {PASSWORD}")),
                multipart_body(vec![multipart_part("text", None, b"   ")]),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = body_text(response).await;
        assert!(body.contains(MSG_EMPTY_MESSAGE));
    }

    #[tokio::test]
    async fn chat_rejects_too_many_files() {
        let mut parts: Vec<Vec<u8>> = (0..9)
            .map(|i| multipart_part("files", Some(&format!("f{i}.png")), TEST_PNG))
            .collect();
        parts.push(multipart_part("text", None, "вопрос".as_bytes()));
        let response = test_router(ok_responder(), None)
            .oneshot(post_chat(
                Some(&format!("Bearer {PASSWORD}")),
                multipart_body(parts),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = body_text(response).await;
        assert!(body.contains(MSG_TOO_MANY_FILES));
    }

    #[tokio::test]
    async fn chat_rejects_bad_history_json() {
        let response = test_router(ok_responder(), None)
            .oneshot(post_chat(
                Some(&format!("Bearer {PASSWORD}")),
                multipart_body(vec![
                    multipart_part("text", None, "привет".as_bytes()),
                    multipart_part("history", None, "не json".as_bytes()),
                ]),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = body_text(response).await;
        assert!(body.contains(MSG_BAD_HISTORY));
    }

    #[tokio::test]
    async fn chat_maps_unsupported_attachment_to_415() {
        let response = test_router(err_responder(ReplyOutcome::Unsupported), None)
            .oneshot(post_chat(
                Some(&format!("Bearer {PASSWORD}")),
                multipart_body(vec![multipart_part(
                    "files",
                    Some("a.bin"),
                    &[0, 1, 2, 0xff],
                )]),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        let body = body_text(response).await;
        assert!(body.contains(MSG_UNSUPPORTED_FILE));
    }

    #[tokio::test]
    async fn chat_maps_too_large_attachment_to_413() {
        let response = test_router(err_responder(ReplyOutcome::TooLarge), None)
            .oneshot(post_chat(
                Some(&format!("Bearer {PASSWORD}")),
                multipart_body(vec![multipart_part("text", None, "вопрос".as_bytes())]),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn chat_maps_upstream_failure_to_502() {
        let response = test_router(err_responder(ReplyOutcome::NoAnswer), None)
            .oneshot(post_chat(
                Some(&format!("Bearer {PASSWORD}")),
                multipart_body(vec![multipart_part("text", None, "вопрос".as_bytes())]),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let body = body_text(response).await;
        assert!(body.contains(MSG_UPSTREAM_FAILED));
    }

    #[tokio::test]
    async fn history_returns_empty_list_without_database() {
        let response = test_router(ok_responder(), None)
            .oneshot(get("/api/history", Some(&format!("Bearer {PASSWORD}"))))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_text(response).await, "[]");
    }

    #[tokio::test]
    async fn ingest_rejects_bad_payload() {
        let response = test_router(ok_responder(), None)
            .oneshot(post_json(
                "/api/ingest",
                Some(&format!("Bearer {PASSWORD}")),
                "{}",
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = body_text(response).await;
        assert!(body.contains(MSG_BAD_INGEST));
    }

    #[tokio::test]
    async fn client_error_accepts_message() {
        let response = test_router(ok_responder(), None)
            .oneshot(post_json(
                "/api/client-error",
                Some(&format!("Bearer {PASSWORD}")),
                r#"{"message":"упало","where":"app","stack":"...","version":"1.0"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn client_error_requires_message() {
        let response = test_router(ok_responder(), None)
            .oneshot(post_json(
                "/api/client-error",
                Some(&format!("Bearer {PASSWORD}")),
                r#"{"message":"  "}"#,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn change_disabled_returns_501() {
        let response = test_router(ok_responder(), None)
            .oneshot(post_json(
                "/api/change",
                Some(&format!("Bearer {PASSWORD}")),
                r#"{"request":"сделай кнопку больше"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
        let body = body_text(response).await;
        assert!(body.contains(MSG_CHANGES_DISABLED));
    }

    #[tokio::test]
    async fn change_accepts_request_with_requester() {
        let response = test_router(ok_responder(), Some(Arc::new(FakeRequester)))
            .oneshot(post_json(
                "/api/change",
                Some(&format!("Bearer {PASSWORD}")),
                r#"{"request":"сделай кнопку больше"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let body = body_text(response).await;
        assert!(body.contains("0a1b2c3d4e5f"));
        assert!(body.contains(CHANGE_ACCEPTED_MESSAGE));
    }

    #[tokio::test]
    async fn change_status_rejects_bad_id() {
        let response = test_router(ok_responder(), Some(Arc::new(FakeRequester)))
            .oneshot(get("/api/change/ZZZ", Some(&format!("Bearer {PASSWORD}"))))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = body_text(response).await;
        assert!(body.contains(MSG_BAD_CHANGE_ID));
    }

    #[tokio::test]
    async fn change_status_returns_state() {
        let response = test_router(ok_responder(), Some(Arc::new(FakeRequester)))
            .oneshot(get(
                "/api/change/0a1b2c3d4e5f",
                Some(&format!("Bearer {PASSWORD}")),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_text(response).await, r#"{"state":"queued"}"#);
    }

    #[tokio::test]
    async fn plan_returns_steps_when_changes_enabled() {
        let response = test_router(ok_responder(), Some(Arc::new(FakeRequester)))
            .oneshot(post_json(
                "/api/change/plan",
                Some(&format!("Bearer {PASSWORD}")),
                r#"{"request":"сделай кнопку больше","history":[]}"#,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            body_text(response).await,
            r#"{"summary":"понял задачу","steps":["шаг один"]}"#
        );
    }

    #[tokio::test]
    async fn cors_preflight_for_allowed_origin() {
        let response = test_router(ok_responder(), None)
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/api/chat")
                    .header("origin", "http://localhost:4200")
                    .header("access-control-request-method", "POST")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("access-control-allow-origin")
                .and_then(|value| value.to_str().ok()),
            Some("http://localhost:4200")
        );
    }

    #[tokio::test]
    async fn responses_carry_json_content_type_and_no_store() {
        let response = test_router(ok_responder(), None)
            .oneshot(get("/api/history", Some(&format!("Bearer {PASSWORD}"))))
            .await
            .unwrap();
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("application/json; charset=utf-8")
        );
        assert_eq!(
            response
                .headers()
                .get("cache-control")
                .and_then(|value| value.to_str().ok()),
            Some("no-store")
        );
    }
}
