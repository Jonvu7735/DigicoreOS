//! Outbox relay ports (DATA-STRATEGY.md §3.2).

use async_trait::async_trait;
use uuid::Uuid;

use crate::message::{OutboxMessage, OutboxResult};

/// Read/clear side of the outbox, used by the relay worker.
#[async_trait]
pub trait OutboxRepository: Send + Sync {
    async fn fetch_unpublished(&self, limit: i64) -> OutboxResult<Vec<OutboxMessage>>;
    async fn mark_published(&self, event_id: &Uuid) -> OutboxResult<()>;
}

/// Raw event-bus publisher (subject + bytes).
#[async_trait]
pub trait RawEventPublisher: Send + Sync {
    async fn publish(&self, subject: &str, payload: &[u8]) -> OutboxResult<()>;
}
