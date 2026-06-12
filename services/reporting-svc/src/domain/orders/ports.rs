//! Ports for the orders read model (implemented in infra/db).

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::orders::entities::{NewOrderFact, OrdersOverview, ReportedOrder};
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

/// Write + read side of the orders projection.
#[async_trait]
pub trait OrdersProjection: Send + Sync {
    /// Project one `OrderCreated`, **idempotently** (`order_id` is the natural
    /// key, so an at-least-once re-delivery is a no-op).
    async fn apply_order_created(&self, fact: &NewOrderFact) -> DomainResult<()>;
    /// Most-recent orders for a tenant (paginated), optionally bounded to
    /// `from <= created_at < to` (each bound is applied only when `Some`).
    async fn list(
        &self,
        tenant: &TenantId,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<ReportedOrder>>;
    /// Count + summed amount for a tenant (zeroed if none).
    async fn overview(&self, tenant: &TenantId) -> DomainResult<OrdersOverview>;
}
