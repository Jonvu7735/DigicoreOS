//! Events PUBLISHED by this vertical. These are the vertical's OWN contract — it
//! reuses the shared `EventHeader` envelope but defines its payloads locally
//! (the shared `event-models` crate stays core/platform-only). Built into an
//! `OutboxMessage` and shipped through the same transactional outbox the core
//! services use.

use event_models::EventHeader;
use platform_outbox::OutboxMessage;
use serde::Serialize;

use crate::domain::shared::error::{DomainError, DomainResult};

/// NATS subjects owned by this vertical (`platform.trade_export.<entity>.<action>`).
pub mod subjects {
    pub const SHIPMENT_BOOKED: &str = "platform.trade_export.shipment.booked";
}

/// An export shipment was booked with a carrier.
#[derive(Debug, Clone, Serialize)]
pub struct ShipmentBooked {
    pub header: EventHeader,
    pub shipment_id: String,
    pub reference: String,
    pub destination_country: String,
    /// The ERP order this shipment fulfils, if any.
    pub order_id: Option<String>,
}

/// Convert a `ShipmentBooked` into an outbox message (header + JSON payload).
pub fn booked_outbox(event: &ShipmentBooked) -> DomainResult<OutboxMessage> {
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
        subject: subjects::SHIPMENT_BOOKED.to_string(),
        payload,
    })
}
