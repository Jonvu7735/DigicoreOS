//! Insight entity (maps to `ai_svc.insights`).

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::shared::types::TenantId;

/// A generated analysis. `source_ref` links back to what triggered it (e.g. a
/// report snapshot id) when the insight came from an event.
#[derive(Debug, Clone)]
pub struct Insight {
    pub id: Uuid,
    pub tenant_id: TenantId,
    /// e.g. `sales_anomaly`, `churn_risk`, `snapshot_digest`.
    pub category: String,
    pub summary: String,
    pub source_ref: Option<String>,
    pub created_at: DateTime<Utc>,
}
