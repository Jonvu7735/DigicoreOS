//! Build a transactional-outbox message from a domain `AuthEvent`.
//!
//! The mechanics (storage, relay, publishing) live in the shared
//! `platform-outbox` crate; this is the only auth-svc–specific glue: turning an
//! `AuthEvent` (header + JSON payload + subject) into an [`OutboxMessage`].

use event_models::auth::AuthEvent;
use platform_outbox::OutboxMessage;

use crate::domain::shared::error::{DomainError, DomainResult};

/// Convert a domain [`AuthEvent`] into an [`OutboxMessage`] ready to be enqueued
/// in the service's state transaction (DATA-STRATEGY.md §3.2).
pub fn outbox_message(event: &AuthEvent) -> DomainResult<OutboxMessage> {
    let header = event.header();
    let bytes = event
        .payload_json()
        .map_err(|e| DomainError::Internal(format!("event serialize failed: {e}")))?;
    let payload = serde_json::from_slice(&bytes)
        .map_err(|e| DomainError::Internal(format!("event to json failed: {e}")))?;
    Ok(OutboxMessage {
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
