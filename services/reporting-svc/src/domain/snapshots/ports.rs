//! Ports for the snapshots context (implemented in infra/db).

use async_trait::async_trait;
use platform_outbox::OutboxMessage;

use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;
use crate::domain::snapshots::entities::Snapshot;

#[async_trait]
pub trait SnapshotRepository: Send + Sync {
    /// Insert the snapshot and enqueue `event` (ReportSnapshotCreated), in one tx.
    async fn create(&self, snapshot: &Snapshot, event: &OutboxMessage) -> DomainResult<()>;
    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Snapshot>>;
}
