//! Events PUBLISHED by this vertical. As with trade-export, payloads are defined
//! locally (the shared `event-models` crate stays core-only) and shipped through
//! the same transactional outbox the core services use.

use event_models::EventHeader;
use platform_outbox::OutboxMessage;
use serde::Serialize;

use crate::domain::shared::error::{DomainError, DomainResult};

/// NATS subjects owned by this vertical (`platform.retail.<entity>.<action>`).
pub mod subjects {
    pub const POINTS_REDEEMED: &str = "platform.retail.points.redeemed";
}

/// A customer redeemed loyalty points.
#[derive(Debug, Clone, Serialize)]
pub struct PointsRedeemed {
    pub header: EventHeader,
    pub customer_id: String,
    pub points_redeemed: i64,
    pub balance_after: i64,
}

/// Convert a `PointsRedeemed` into an outbox message (header + JSON payload).
pub fn redeemed_outbox(event: &PointsRedeemed) -> DomainResult<OutboxMessage> {
    let payload = serde_json::to_value(event)
        .map_err(|e| DomainError::Internal(format!("event serialize failed: {e}")))?;
    Ok(OutboxMessage {
        event_id: event.header.event_id,
        occurred_at: event.header.occurred_at,
        tenant_id: event.header.tenant_id.clone(),
        aggregate_type: event.header.aggregate_type.clone(),
        aggregate_id: event.header.aggregate_id.clone(),
        event_type: event.header.event_type.clone(),
        version: event.header.version,
        subject: subjects::POINTS_REDEEMED.to_string(),
        payload,
    })
}
