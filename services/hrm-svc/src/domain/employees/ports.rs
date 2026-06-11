//! Ports for the employees context (implemented in infra/db).

use async_trait::async_trait;
use platform_outbox::OutboxMessage;
use uuid::Uuid;

use crate::domain::employees::entities::Employee;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

#[async_trait]
pub trait EmployeeRepository: Send + Sync {
    /// Insert the employee and enqueue `event` (EmployeeHired), in one transaction.
    async fn create(&self, employee: &Employee, event: &OutboxMessage) -> DomainResult<()>;
    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Employee>>;
    async fn find_in_tenant(&self, tenant: &TenantId, id: &Uuid) -> DomainResult<Option<Employee>>;
    /// Persist a status change and enqueue `event` (EmployeeTerminated), in one tx.
    async fn save_status(&self, employee: &Employee, event: &OutboxMessage) -> DomainResult<()>;
}
