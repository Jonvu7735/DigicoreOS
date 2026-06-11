//! Deal DTOs (`/api/v1/crm/deals`).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::deals::entities::Deal;

#[derive(Debug, Deserialize)]
pub struct CreateDealRequest {
    pub customer_id: Uuid,
    pub title: String,
    /// Estimated value in minor currency units.
    pub amount_estimate: i64,
}

#[derive(Debug, Deserialize)]
pub struct ChangeStageRequest {
    /// Target stage: `QUALIFIED` | `PROPOSAL` | `WON` | `LOST`.
    pub stage: String,
}

#[derive(Debug, Serialize)]
pub struct DealResponse {
    pub id: String,
    pub tenant_id: String,
    pub customer_id: String,
    pub title: String,
    pub amount_estimate: i64,
    pub stage: String,
    pub created_at: String,
}

impl From<Deal> for DealResponse {
    fn from(d: Deal) -> Self {
        Self {
            id: d.id.to_string(),
            tenant_id: d.tenant_id.0,
            customer_id: d.customer_id.to_string(),
            title: d.title,
            amount_estimate: d.amount_estimate.0,
            stage: d.stage.as_str().to_string(),
            created_at: d.created_at.to_rfc3339(),
        }
    }
}
