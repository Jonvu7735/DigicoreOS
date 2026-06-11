//! Ports for the inventory context.

use async_trait::async_trait;
use platform_outbox::OutboxMessage;

use crate::domain::inventory::entities::{StockAdjustment, StockLevel};
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

#[async_trait]
pub trait InventoryRepository: Send + Sync {
    /// Apply `adjustment` to the stock level (upsert += delta), append the
    /// movement, and enqueue `event` — all in one transaction. Returns the new
    /// on-hand quantity; rejects (rolls back) if it would go negative.
    async fn adjust(
        &self,
        adjustment: &StockAdjustment,
        event: &OutboxMessage,
    ) -> DomainResult<i64>;
    async fn list_stock(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<StockLevel>>;
    async fn list_adjustments(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<StockAdjustment>>;
}
