//! Inventory entities (map to `erp_core_svc.stock_levels` / `stock_adjustments`).

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::shared::types::TenantId;

/// Current on-hand quantity of a product in a warehouse.
#[derive(Debug, Clone)]
pub struct StockLevel {
    pub tenant_id: TenantId,
    pub product_id: Uuid,
    pub warehouse_id: String,
    pub quantity: i64,
}

/// An append-only stock movement.
#[derive(Debug, Clone)]
pub struct StockAdjustment {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub product_id: Uuid,
    pub warehouse_id: String,
    /// Signed quantity delta (positive = increase).
    pub delta: i64,
    /// e.g. `order`, `manual_adjustment`.
    pub reason: String,
    pub created_at: DateTime<Utc>,
}
