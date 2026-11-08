use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_postgres::types::ToSql;

use super::constants::{
    CLOSE_TIMEOUT, MAX_STORED_TEXT_BYTES, QUEUE_SIZE, READ_TIMEOUT, WRITE_TIMEOUT, clamp_limit,
    clip,
};
use super::models::{ErrorEntry, Event, IncomingMessage, Message, MessageFilter, StoredFile};
use super::{Store, StoreError};

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "001_init.sql",
        include_str!("../../migrations/001_init.sql"),
    ),
    (
        "002_notes.sql",
        include_str!("../../migrations/002_notes.sql"),
    ),
    (
        "003_attachments.sql",
        include_str!("../../migrations/003_attachments.sql"),
    ),
];

const ADVISORY_LOCK_KEY: i64 = 7283451;

enum Job {
    Messages(Vec<IncomingMessage>),
    Attachments {
        conversation_id: String,
        turn_id: String,
        files: Vec<StoredFile>,
    },
    Event {
        kind: String,
        reference: String,
        payload: Value,
    },
    Error {
        source: String,
        message: String,
        details: Value,
    },
}

pub struct Postgres {
    pool: deadpool_postgres::Pool,
    sender: Mutex<Option<mpsc::Sender<Job>>>,
    worker: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl Postgres {
    pub async fn open(database_url: &str) -> Result<Postgres, StoreError> {
        let (url, search_path) = split_search_path(database_url);
        let mut config: tokio_postgres::Config = url
            .parse()
            .map_err(|err| StoreError::Config(format!("parse {url}: {err}")))?;
        if let Some(search_path) = search_path.as_deref() {
            config.options(format!("-csearch_path={search_path}"));
        }
        let manager = deadpool_postgres::Manager::new(config, tokio_postgres::NoTls);
        let pool = deadpool_postgres::Pool::builder(manager)
            .max_size(4)
            .build()
            .map_err(|err| StoreError::Config(format!("build pool: {err}")))?;

        let connection = pool.get().await?;
        connection.execute("SELECT 1", &[]).await?;
        drop(connection);

        migrate(&pool).await?;

        let (sender, receiver) = mpsc::channel(QUEUE_SIZE);
        let worker = tokio::spawn(work(receiver, pool.clone()));
        Ok(Postgres {
            pool,
            sender: Mutex::new(Some(sender)),
            worker: Mutex::new(Some(worker)),
        })
    }

    fn enqueue(&self, job: Job) {
        let guard = self.sender.lock().expect("store sender lock");
        match guard.as_ref() {
            Some(sender) => {
                if let Err(err) = sender.try_send(job) {
                    tracing::warn!(error = %err, "database queue is full, record dropped");
                }
            }
            None => tracing::warn!("database queue is closed, record dropped"),
        }
    }
}

async fn work(mut receiver: mpsc::Receiver<Job>, pool: deadpool_postgres::Pool) {
    while let Some(job) = receiver.recv().await {
        let result = match tokio::time::timeout(WRITE_TIMEOUT, handle_job(&pool, job)).await {
            Ok(result) => result,
            Err(elapsed) => Err(StoreError::Config(format!("write timeout: {elapsed}"))),
        };
        if let Err(err) = result {
            tracing::warn!(error = %err, "database write failed");
        }
    }
}

async fn handle_job(pool: &deadpool_postgres::Pool, job: Job) -> Result<(), StoreError> {
    let connection = pool.get().await?;
    match job {
        Job::Messages(messages) => {
            for message in messages {
                let attachments = Value::Array(
                    message
                        .attachments_names
                        .into_iter()
                        .map(Value::String)
                        .collect(),
                );
                let meta = message.meta.unwrap_or_else(|| json!({}));
                let params: [&(dyn ToSql + Sync); 8] = [
                    &message.source,
                    &message.conversation_id,
                    &message.external_id,
                    &message.role,
                    &clip(&message.text, MAX_STORED_TEXT_BYTES),
                    &attachments,
                    &meta,
                    &message.created_at,
                ];
                connection
                    .execute(
                        "INSERT INTO messages (source, conversation_id, external_id, role, text, attachments, meta, created_at)
                        VALUES ($1, $2, NULLIF($3, ''), $4, $5, $6, $7, COALESCE($8, now()))
                        ON CONFLICT (source, external_id) WHERE external_id IS NOT NULL DO NOTHING",
                        &params,
                    )
                    .await?;
            }
        }
        Job::Attachments {
            conversation_id,
            turn_id,
            files,
        } => {
            for file in files {
                connection
                    .execute(
                        "INSERT INTO attachments (conversation_id, turn_id, name, data) VALUES ($1, $2, $3, $4)",
                        &[&conversation_id, &turn_id, &file.name, &file.data],
                    )
                    .await?;
            }
        }
        Job::Event {
            kind,
            reference,
            payload,
        } => {
            connection
                .execute(
                    "INSERT INTO events (kind, ref, payload) VALUES ($1, $2, $3)",
                    &[&kind, &reference, &payload],
                )
                .await?;
        }
        Job::Error {
            source,
            message,
            details,
        } => {
            let clipped = clip(&message, MAX_STORED_TEXT_BYTES);
            connection
                .execute(
                    "INSERT INTO error_log (source, message, details) VALUES ($1, $2, $3)",
                    &[&source, &clipped, &details],
                )
                .await?;
        }
    }
    Ok(())
}

async fn migrate(pool: &deadpool_postgres::Pool) -> Result<(), StoreError> {
    let mut connection = pool.get().await?;
    connection
        .execute("SELECT pg_advisory_lock($1)", &[&ADVISORY_LOCK_KEY])
        .await?;
    let result = migrate_locked(&mut connection).await;
    let _ = connection
        .execute("SELECT pg_advisory_unlock($1)", &[&ADVISORY_LOCK_KEY])
        .await;
    result
}

async fn migrate_locked(connection: &mut deadpool_postgres::Object) -> Result<(), StoreError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                name TEXT PRIMARY KEY,
                applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )",
            &[],
        )
        .await?;

    for (name, sql_text) in MIGRATIONS {
        let row = connection
            .query_one(
                "SELECT EXISTS (SELECT 1 FROM schema_migrations WHERE name = $1)",
                &[name],
            )
            .await?;
        let applied: bool = row.get(0);
        if applied {
            continue;
        }
        let transaction = connection.transaction().await?;
        transaction.execute(*sql_text, &[]).await?;
        transaction
            .execute("INSERT INTO schema_migrations (name) VALUES ($1)", &[name])
            .await?;
        transaction.commit().await?;
    }
    Ok(())
}

fn parse_turn_id(meta: &Value) -> String {
    meta.get("turn")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub fn message_turn_id(meta: &Value) -> String {
    parse_turn_id(meta)
}

#[async_trait::async_trait]
impl Store for Postgres {
    fn save_messages(&self, messages: Vec<IncomingMessage>) {
        if messages.is_empty() {
            return;
        }
        self.enqueue(Job::Messages(messages));
    }

    fn save_event(&self, kind: &str, reference: &str, payload: Value) {
        self.enqueue(Job::Event {
            kind: kind.to_string(),
            reference: reference.to_string(),
            payload,
        });
    }

    fn save_error(&self, source: &str, message: &str, details: Value) {
        self.enqueue(Job::Error {
            source: source.to_string(),
            message: message.to_string(),
            details,
        });
    }

    fn save_attachments(&self, conversation_id: &str, turn_id: &str, files: Vec<StoredFile>) {
        if files.is_empty() {
            return;
        }
        self.enqueue(Job::Attachments {
            conversation_id: conversation_id.to_string(),
            turn_id: turn_id.to_string(),
            files,
        });
    }

    async fn messages(&self, filter: &MessageFilter) -> Result<Vec<Message>, StoreError> {
        let connection = read_connection(&self.pool).await?;
        let rows = tokio::time::timeout(READ_TIMEOUT, async {
            connection
                .query(
                    "SELECT id, source, conversation_id, COALESCE(external_id, '') AS external_id, role, text, attachments, meta, created_at
                    FROM (
                        SELECT * FROM messages
                        WHERE ($1 = '' OR source = $1) AND ($2 = '' OR conversation_id = $2)
                        ORDER BY created_at DESC, id DESC
                        LIMIT $3
                    ) recent
                    ORDER BY created_at, id",
                    &[&filter.source, &filter.conversation_id, &clamp_limit(filter.limit)],
                )
                .await
        })
        .await
        .map_err(|elapsed| StoreError::Config(format!("read timeout: {elapsed}")))??;

        let mut messages = Vec::with_capacity(rows.len());
        for row in rows {
            let attachments: Value = row.get("attachments");
            messages.push(Message {
                id: row.get("id"),
                source: row.get("source"),
                conversation_id: row.get("conversation_id"),
                external_id: row.get("external_id"),
                role: row.get("role"),
                text: row.get("text"),
                attachments: attachments
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default(),
                meta: row.get("meta"),
                created_at: row.get::<_, DateTime<Utc>>("created_at"),
            });
        }
        Ok(messages)
    }

    async fn events(&self, kind: &str, limit: i64) -> Result<Vec<Event>, StoreError> {
        let connection = read_connection(&self.pool).await?;
        let rows = tokio::time::timeout(READ_TIMEOUT, async {
            connection
                .query(
                    "SELECT id, kind, ref, payload, created_at FROM events
                    WHERE ($1 = '' OR kind = $1) ORDER BY created_at DESC, id DESC LIMIT $2",
                    &[&kind, &clamp_limit(limit)],
                )
                .await
        })
        .await
        .map_err(|elapsed| StoreError::Config(format!("read timeout: {elapsed}")))??;

        Ok(rows
            .into_iter()
            .map(|row| Event {
                id: row.get("id"),
                kind: row.get("kind"),
                reference: row.get("ref"),
                payload: row.get("payload"),
                created_at: row.get("created_at"),
            })
            .collect())
    }

    async fn errors(&self, limit: i64) -> Result<Vec<ErrorEntry>, StoreError> {
        let connection = read_connection(&self.pool).await?;
        let rows = tokio::time::timeout(READ_TIMEOUT, async {
            connection
                .query(
                    "SELECT id, source, message, details, created_at FROM error_log
                    ORDER BY created_at DESC, id DESC LIMIT $1",
                    &[&clamp_limit(limit)],
                )
                .await
        })
        .await
        .map_err(|elapsed| StoreError::Config(format!("read timeout: {elapsed}")))??;

        Ok(rows
            .into_iter()
            .map(|row| ErrorEntry {
                id: row.get("id"),
                source: row.get("source"),
                message: row.get("message"),
                details: row.get("details"),
                created_at: row.get("created_at"),
            })
            .collect())
    }

    async fn attachments(
        &self,
        conversation_id: &str,
        turn_ids: &[String],
        max_bytes: i32,
    ) -> Result<Vec<StoredFile>, StoreError> {
        if conversation_id.is_empty() || turn_ids.is_empty() {
            return Ok(Vec::new());
        }
        let connection = read_connection(&self.pool).await?;
        let rows = tokio::time::timeout(READ_TIMEOUT, async {
            connection
                .query(
                    "SELECT turn_id, name, data FROM attachments
                    WHERE conversation_id = $1 AND turn_id = ANY($2) AND octet_length(data) <= $3
                    ORDER BY id",
                    &[&conversation_id, &turn_ids.to_vec(), &max_bytes],
                )
                .await
        })
        .await
        .map_err(|elapsed| StoreError::Config(format!("read timeout: {elapsed}")))??;

        Ok(rows
            .into_iter()
            .map(|row| StoredFile {
                turn_id: row.get("turn_id"),
                name: row.get("name"),
                data: row.get("data"),
            })
            .collect())
    }

    async fn close(&self) {
        let sender = self.sender.lock().expect("store sender lock").take();
        drop(sender);
        let worker = self.worker.lock().expect("store worker lock").take();
        if let Some(worker) = worker {
            let _ = tokio::time::timeout(CLOSE_TIMEOUT, worker).await;
        }
        self.pool.close();
    }
}

async fn read_connection(
    pool: &deadpool_postgres::Pool,
) -> Result<deadpool_postgres::Object, StoreError> {
    pool.get().await.map_err(StoreError::from)
}

fn split_search_path(raw: &str) -> (String, Option<String>) {
    let Some((base, query)) = raw.split_once('?') else {
        return (raw.to_string(), None);
    };
    let mut kept: Vec<&str> = Vec::new();
    let mut search_path = None;
    for pair in query.split('&') {
        if let Some(value) = pair.strip_prefix("search_path=") {
            search_path = Some(value.to_string());
        } else if !pair.is_empty() {
            kept.push(pair);
        }
    }
    let mut url = base.to_string();
    if !kept.is_empty() {
        url.push('?');
        url.push_str(&kept.join("&"));
    }
    (url, search_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_id_extracted_from_meta() {
        let meta = json!({"turn": "abc123"});
        assert_eq!(message_turn_id(&meta), "abc123");
        assert_eq!(message_turn_id(&json!({})), "");
    }

    #[test]
    fn split_search_path_moves_param_to_options() {
        let (url, search_path) = split_search_path(
            "postgres://user:pass@127.0.0.1:5432/db?sslmode=disable&search_path=advisor",
        );
        assert_eq!(
            url,
            "postgres://user:pass@127.0.0.1:5432/db?sslmode=disable"
        );
        assert_eq!(search_path.as_deref(), Some("advisor"));

        let (url, search_path) = split_search_path("postgres://user@localhost/db");
        assert_eq!(url, "postgres://user@localhost/db");
        assert_eq!(search_path, None);
    }
}
