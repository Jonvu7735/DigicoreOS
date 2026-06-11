//! Ports for the customers context (implemented in infra/db).

use async_trait::async_trait;
use platform_outbox::OutboxMessage;
use uuid::Uuid;

use crate::domain::customers::entities::Customer;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

#[async_trait]
pub trait CustomerRepository: Send + Sync {
    /// Insert the customer and enqueue `event` (CustomerCreated), in one transaction.
    async fn create(&self, customer: &Customer, event: &OutboxMessage) -> DomainResult<()>;
    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Customer>>;
    async fn find_in_tenant(&self, tenant: &TenantId, id: &Uuid) -> DomainResult<Option<Customer>>;
    /// Persist field changes and enqueue `event` (CustomerUpdated), in one transaction.
    async fn update(&self, customer: &Customer, event: &OutboxMessage) -> DomainResult<()>;
}
