//! Transactional outbox value object (DATA-STRATEGY.md §3.2).
//!
//! Business events are written to `auth_svc.outbox_events` in the SAME
//! transaction as the state change; a relay worker later publishes them to NATS
//! and marks them sent. This guarantees state and events commit together.

use chrono::{DateTime, Utc};
use event_models::auth::AuthEvent;
use uuid::Uuid;

use crate::domain::shared::error::{DomainError, DomainResult};

/// One event queued in (or read from) the outbox.
#[derive(Debug, Clone)]
pub struct OutboxMessage {
    /// Unique id (also the outbox row PK and the NATS idempotency key).
    pub event_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub tenant_id: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub version: i32,
    pub subject: String,
    /// JSON payload published verbatim to the bus.
    pub payload: serde_json::Value,
}

impl OutboxMessage {
    /// Build from a domain `AuthEvent` (header + serialized payload).
    pub fn from_auth_event(event: &AuthEvent) -> DomainResult<Self> {
        let header = event.header();
        let bytes = event
            .payload_json()
            .map_err(|e| DomainError::Internal(format!("event serialize failed: {e}")))?;
        let payload = serde_json::from_slice(&bytes)
            .map_err(|e| DomainError::Internal(format!("event to json failed: {e}")))?;
        Ok(Self {
            event_id: header.event_id,
            occurred_at: header.occurred_at,
            tenant_id: header.tenant_id.clone(),
            aggregate_type: header.aggregate_type.clone(),
            aggregate_id: header.aggregate_id.clone(),
            event_type: header.event_type.clone(),
            version: header.version,
            subject: event.subject().to_string(),
            payload,
        })
    }

    /// Payload as bytes for publishing to the bus.
    pub fn payload_bytes(&self) -> Vec<u8> {
        // `Value` always re-serializes; the unwrap cannot fail.
        serde_json::to_vec(&self.payload).unwrap_or_default()
    }
}
