//! Ports for the deals read model (implemented in infra/db).

use async_trait::async_trait;

use crate::domain::deals::entities::{DealStageChange, NewDealFact, StageCount};
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

/// Write + read side of the deals projection (CRM funnel).
#[async_trait]
pub trait DealsProjection: Send + Sync {
    /// Project one `DealCreated`, **idempotently** (`deal_id` is the natural
    /// key, so an at-least-once re-delivery is a no-op).
    async fn apply_deal_created(&self, fact: &NewDealFact) -> DomainResult<()>;
    /// Apply a `DealStageChanged`, **monotonically** by event time: a stale or
    /// duplicate change never regresses the stored stage.
    async fn apply_deal_stage_changed(&self, change: &DealStageChange) -> DomainResult<()>;
    /// Deal count per current stage for a tenant (the funnel).
    async fn funnel(&self, tenant: &TenantId) -> DomainResult<Vec<StageCount>>;
}
