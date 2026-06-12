//! Ports for the customers read model (implemented in infra/db).

use async_trait::async_trait;

use crate::domain::customers::entities::{NewCustomerFact, ReportedCustomer};
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

/// Write + read side of the customers projection.
#[async_trait]
pub trait CustomersProjection: Send + Sync {
    /// Project one `CustomerCreated`, **idempotently** (`customer_id` is the
    /// natural key, so an at-least-once re-delivery is a no-op).
    async fn apply_customer_created(&self, fact: &NewCustomerFact) -> DomainResult<()>;
    /// Most-recently-created customers for a tenant (paginated).
    async fn list(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<ReportedCustomer>>;
    /// Total customers for a tenant (zeroed if none).
    async fn count(&self, tenant: &TenantId) -> DomainResult<i64>;
}
