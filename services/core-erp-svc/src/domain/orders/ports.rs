//! Ports for the orders context. Mutations also enqueue an event into the
//! outbox in the SAME transaction (DATA-STRATEGY.md §3.2).

use async_trait::async_trait;
use platform_outbox::OutboxMessage;
use uuid::Uuid;

use crate::domain::orders::entities::Order;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

#[async_trait]
pub trait OrderRepository: Send + Sync {
    /// Insert the order and enqueue `event`, in one transaction.
    async fn create(&self, order: &Order, event: &OutboxMessage) -> DomainResult<()>;
    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Order>>;
    async fn find_in_tenant(&self, tenant: &TenantId, id: &Uuid) -> DomainResult<Option<Order>>;
    /// Persist the order's new status and enqueue `event`, in one transaction.
    async fn save_status(&self, order: &Order, event: &OutboxMessage) -> DomainResult<()>;
}
