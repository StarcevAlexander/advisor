use std::sync::Arc;

use serde_json::{Map, Value, json};
use tokio::sync::OnceCell;

use super::Store;
use super::constants::ERROR_SOURCE_SERVER;

pub struct ErrorLogLayer {
    store: Arc<OnceCell<Arc<dyn Store>>>,
}

impl ErrorLogLayer {
    pub fn new(store: Arc<OnceCell<Arc<dyn Store>>>) -> Self {
        ErrorLogLayer { store }
    }
}

impl<S> tracing_subscriber::Layer<S> for ErrorLogLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if *event.metadata().level() != tracing::Level::ERROR {
            return;
        }
        let Some(store) = self.store.get() else {
            return;
        };
        let mut details = Map::new();
        details.insert("level".to_string(), json!("ERROR"));
        let mut message = String::new();
        let mut visitor = FieldVisitor {
            details: &mut details,
            message: &mut message,
        };
        event.record(&mut visitor);
        store.save_error(ERROR_SOURCE_SERVER, &message, Value::Object(details));
    }
}

struct FieldVisitor<'a> {
    details: &'a mut Map<String, Value>,
    message: &'a mut String,
}

impl tracing::field::Visit for FieldVisitor<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let value = format!("{value:?}");
        if field.name() == "message" {
            *self.message = value;
        } else {
            self.details
                .insert(field.name().to_string(), Value::String(value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            *self.message = value.to_string();
        } else {
            self.details
                .insert(field.name().to_string(), Value::String(value.to_string()));
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.details.insert(field.name().to_string(), json!(value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.details.insert(field.name().to_string(), json!(value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.details.insert(field.name().to_string(), json!(value));
    }
}
