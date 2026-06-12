//! Ports for the inventory read model (implemented in infra/db).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::inventory::entities::{StockAdjustment, StockLevel};
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

/// Write + read side of the inventory projection.
#[async_trait]
pub trait InventoryProjection: Send + Sync {
    /// Apply one `StockAdjusted`, **idempotently by `event_id`** (the delta is
    /// additive, so re-applying a delivered event would double-count without the
    /// `processed_events` guard).
    async fn apply_stock_adjusted(
        &self,
        event_id: Uuid,
        tenant: &TenantId,
        adj: &StockAdjustment,
    ) -> DomainResult<()>;
    /// Current stock-on-hand per (product, warehouse) for a tenant.
    async fn summary(&self, tenant: &TenantId) -> DomainResult<Vec<StockLevel>>;
}
