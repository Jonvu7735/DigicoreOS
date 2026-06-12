//! Ports for the orders read model (implemented in infra/db).

use async_trait::async_trait;

use crate::domain::orders::entities::{NewOrderFact, OrdersOverview, ReportedOrder};
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

/// Write + read side of the orders projection.
#[async_trait]
pub trait OrdersProjection: Send + Sync {
    /// Project one `OrderCreated`, **idempotently** (`order_id` is the natural
    /// key, so an at-least-once re-delivery is a no-op).
    async fn apply_order_created(&self, fact: &NewOrderFact) -> DomainResult<()>;
    /// Most-recent orders for a tenant (paginated).
    async fn list(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<ReportedOrder>>;
    /// Count + summed amount for a tenant (zeroed if none).
    async fn overview(&self, tenant: &TenantId) -> DomainResult<OrdersOverview>;
}
