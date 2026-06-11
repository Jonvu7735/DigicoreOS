//! The outbox message value + error type.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Outbox storage / publish failure.
#[derive(Debug, thiserror::Error)]
pub enum OutboxError {
    #[error("outbox storage error: {0}")]
    Storage(String),
    #[error("publish error: {0}")]
    Publish(String),
}

pub type OutboxResult<T> = Result<T, OutboxError>;

/// One event queued in (or read from) the outbox. Services construct this from
/// their own event enum (header + subject + JSON payload).
#[derive(Debug, Clone)]
pub struct OutboxMessage {
    /// Unique id (outbox row PK and the bus idempotency key).
    pub event_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub tenant_id: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub version: i32,
    pub subject: String,
    pub payload: serde_json::Value,
}

impl OutboxMessage {
    /// Payload as bytes for publishing to the bus.
    pub fn payload_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&self.payload).unwrap_or_default()
    }
}
