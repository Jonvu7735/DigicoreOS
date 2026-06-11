//! Convert a domain `ErpEvent` into an outbox message (header + JSON payload).

use event_models::erp::ErpEvent;
use platform_outbox::OutboxMessage;

use crate::domain::shared::error::{DomainError, DomainResult};

pub fn outbox_message(event: &ErpEvent) -> DomainResult<OutboxMessage> {
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
