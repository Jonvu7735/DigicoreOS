//! Snapshot DTOs (`/api/v1/reporting/snapshots`).

use serde::{Deserialize, Serialize};

use crate::domain::snapshots::entities::Snapshot;

#[derive(Debug, Deserialize)]
pub struct CreateSnapshotRequest {
    /// e.g. `sales`.
    pub snapshot_type: String,
}

#[derive(Debug, Serialize)]
pub struct SnapshotResponse {
    pub id: String,
    pub tenant_id: String,
    pub snapshot_type: String,
    pub payload: serde_json::Value,
    pub created_at: String,
}

impl From<Snapshot> for SnapshotResponse {
    fn from(s: Snapshot) -> Self {
        Self {
            id: s.id.to_string(),
            tenant_id: s.tenant_id.0,
            snapshot_type: s.snapshot_type,
            payload: s.payload,
            created_at: s.created_at.to_rfc3339(),
        }
    }
}
