//! Ports for the attendance context (implemented in infra/db).

use async_trait::async_trait;
use platform_outbox::OutboxMessage;
use uuid::Uuid;

use crate::domain::attendance::entities::AttendanceRecord;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

#[async_trait]
pub trait AttendanceRepository: Send + Sync {
    /// Insert the record and enqueue `event` (AttendanceRecorded), in one transaction.
    async fn create(&self, record: &AttendanceRecord, event: &OutboxMessage) -> DomainResult<()>;
    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<AttendanceRecord>>;
    async fn find_in_tenant(
        &self,
        tenant: &TenantId,
        id: &Uuid,
    ) -> DomainResult<Option<AttendanceRecord>>;
}
