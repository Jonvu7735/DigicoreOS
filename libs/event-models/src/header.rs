//! Common header carried by every business event (`EVENTS.md` §2.4).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Common envelope fields shared by all platform events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventHeader {
    /// Unique id of this event instance (used for idempotent consumers).
    pub event_id: Uuid,
    /// When the event occurred (UTC).
    pub occurred_at: DateTime<Utc>,
    /// Tenant the event belongs to.
    pub tenant_id: String,
    /// Aggregate kind: `user`, `tenant`, `order`, `customer`, `employee`, ...
    pub aggregate_type: String,
    /// Id of the aggregate instance.
    pub aggregate_id: String,
    /// Event type name, e.g. `UserRegistered`, `OrderCreated`.
    pub event_type: String,
    /// Schema version of the payload (starts at 1).
    pub version: i32,
}

impl EventHeader {
    /// Build a v1 header. `event_id` and `occurred_at` are supplied by the caller
    /// (domain layer owns time/ids via its `Clock` / id-generator ports).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_id: Uuid,
        occurred_at: DateTime<Utc>,
        tenant_id: impl Into<String>,
        aggregate_type: impl Into<String>,
        aggregate_id: impl Into<String>,
        event_type: impl Into<String>,
        version: i32,
    ) -> Self {
        Self {
            event_id,
            occurred_at,
            tenant_id: tenant_id.into(),
            aggregate_type: aggregate_type.into(),
            aggregate_id: aggregate_id.into(),
            event_type: event_type.into(),
            version,
        }
    }
}
