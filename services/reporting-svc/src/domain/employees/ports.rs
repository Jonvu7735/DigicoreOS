//! Ports for the employees read model (implemented in infra/db).

use async_trait::async_trait;

use crate::domain::employees::entities::{NewEmployeeFact, ReportedEmployee};
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

/// Write + read side of the employees projection.
#[async_trait]
pub trait EmployeesProjection: Send + Sync {
    /// Project one `EmployeeHired`, **idempotently** (`employee_id` is the
    /// natural key, so an at-least-once re-delivery is a no-op).
    async fn apply_employee_hired(&self, fact: &NewEmployeeFact) -> DomainResult<()>;
    /// Most-recently-hired employees for a tenant (paginated).
    async fn list(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<ReportedEmployee>>;
    /// Total headcount for a tenant (zeroed if none).
    async fn count(&self, tenant: &TenantId) -> DomainResult<i64>;
}
