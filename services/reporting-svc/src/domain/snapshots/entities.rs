//! Report snapshot entity (maps to `reporting_svc.snapshots`).

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::shared::types::TenantId;

/// A captured read-model state at a point in time. `payload` is the frozen
/// JSON view (e.g. the sales summary) for the snapshot's `snapshot_type`.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub snapshot_type: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
