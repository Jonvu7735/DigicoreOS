//! Ports for the deals context (implemented in infra/db).

use async_trait::async_trait;
use platform_outbox::OutboxMessage;
use uuid::Uuid;

use crate::domain::deals::entities::Deal;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

#[async_trait]
pub trait DealRepository: Send + Sync {
    /// Insert the deal and enqueue `event` (DealCreated), in one transaction.
    async fn create(&self, deal: &Deal, event: &OutboxMessage) -> DomainResult<()>;
    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Deal>>;
    async fn find_in_tenant(&self, tenant: &TenantId, id: &Uuid) -> DomainResult<Option<Deal>>;
    /// Persist a stage change and enqueue `event` (DealStageChanged), in one tx.
    async fn save_stage(&self, deal: &Deal, event: &OutboxMessage) -> DomainResult<()>;
}
