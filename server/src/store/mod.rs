mod constants;
mod error_layer;
mod models;
mod postgres;

pub use constants::*;
pub use error_layer::*;
pub use models::*;
pub use postgres::*;

use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database config: {0}")]
    Config(String),
    #[error("database pool: {0}")]
    Pool(#[source] deadpool_postgres::PoolError),
    #[error("database: {0}")]
    Db(#[source] tokio_postgres::Error),
}

impl From<deadpool_postgres::PoolError> for StoreError {
    fn from(error: deadpool_postgres::PoolError) -> Self {
        StoreError::Pool(error)
    }
}

impl From<tokio_postgres::Error> for StoreError {
    fn from(error: tokio_postgres::Error) -> Self {
        StoreError::Db(error)
    }
}

#[async_trait::async_trait]
pub trait Store: Send + Sync {
    fn save_messages(&self, messages: Vec<IncomingMessage>);
    fn save_event(&self, kind: &str, reference: &str, payload: Value);
    fn save_error(&self, source: &str, message: &str, details: Value);
    fn save_attachments(&self, conversation_id: &str, turn_id: &str, files: Vec<StoredFile>);

    async fn messages(&self, filter: &MessageFilter) -> Result<Vec<Message>, StoreError>;
    async fn events(&self, kind: &str, limit: i64) -> Result<Vec<Event>, StoreError>;
    async fn errors(&self, limit: i64) -> Result<Vec<ErrorEntry>, StoreError>;
    async fn attachments(
        &self,
        conversation_id: &str,
        turn_ids: &[String],
        max_bytes: i32,
    ) -> Result<Vec<StoredFile>, StoreError>;

    async fn close(&self);
}

pub struct Noop;

#[async_trait::async_trait]
impl Store for Noop {
    fn save_messages(&self, _messages: Vec<IncomingMessage>) {}
    fn save_event(&self, _kind: &str, _reference: &str, _payload: Value) {}
    fn save_error(&self, _source: &str, _message: &str, _details: Value) {}
    fn save_attachments(&self, _conversation_id: &str, _turn_id: &str, _files: Vec<StoredFile>) {}

    async fn messages(&self, _filter: &MessageFilter) -> Result<Vec<Message>, StoreError> {
        Ok(Vec::new())
    }
    async fn events(&self, _kind: &str, _limit: i64) -> Result<Vec<Event>, StoreError> {
        Ok(Vec::new())
    }
    async fn errors(&self, _limit: i64) -> Result<Vec<ErrorEntry>, StoreError> {
        Ok(Vec::new())
    }
    async fn attachments(
        &self,
        _conversation_id: &str,
        _turn_ids: &[String],
        _max_bytes: i32,
    ) -> Result<Vec<StoredFile>, StoreError> {
        Ok(Vec::new())
    }

    async fn close(&self) {}
}
