//! Ports for the sales read model (implemented in infra/db).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::sales::entities::SalesSummary;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

/// Write + read side of the sales projection.
#[async_trait]
pub trait SalesProjection: Send + Sync {
    /// Apply one `OrderPaid` event, **idempotently**: the projection records
    /// `event_id` and ignores re-deliveries (NATS is at-least-once).
    async fn apply_order_paid(
        &self,
        event_id: Uuid,
        tenant: &TenantId,
        amount_paid: i64,
    ) -> DomainResult<()>;

    /// Current rollup for a tenant (zeroed if no events yet).
    async fn get_summary(&self, tenant: &TenantId) -> DomainResult<SalesSummary>;
}
