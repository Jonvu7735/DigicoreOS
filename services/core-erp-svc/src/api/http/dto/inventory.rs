//! Inventory DTOs (`/api/v1/erp/inventory`).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::inventory::entities::{StockAdjustment, StockLevel};

#[derive(Debug, Deserialize)]
pub struct CreateAdjustmentRequest {
    pub product_id: Uuid,
    pub warehouse_id: String,
    /// Signed quantity delta (positive = increase).
    pub delta: i64,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct StockLevelResponse {
    pub product_id: String,
    pub warehouse_id: String,
    pub quantity: i64,
}

impl From<StockLevel> for StockLevelResponse {
    fn from(s: StockLevel) -> Self {
        Self {
            product_id: s.product_id.to_string(),
            warehouse_id: s.warehouse_id,
            quantity: s.quantity,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AdjustmentResponse {
    pub id: String,
    pub tenant_id: String,
    pub product_id: String,
    pub warehouse_id: String,
    pub delta: i64,
    pub reason: String,
    pub created_at: String,
    /// On-hand quantity after this adjustment (present only on create).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resulting_quantity: Option<i64>,
}

pub fn adjustment_response(
    a: StockAdjustment,
    resulting_quantity: Option<i64>,
) -> AdjustmentResponse {
    AdjustmentResponse {
        id: a.id.to_string(),
        tenant_id: a.tenant_id.0,
        product_id: a.product_id.to_string(),
        warehouse_id: a.warehouse_id,
        delta: a.delta,
        reason: a.reason,
        created_at: a.created_at.to_rfc3339(),
        resulting_quantity,
    }
}
