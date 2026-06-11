//! Ports for the leave context (implemented in infra/db).
//!
//! Leave has no event contract in EVENTS.md, so the repository is plain CRUD
//! (no outbox).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::leave::entities::LeaveRequest;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

#[async_trait]
pub trait LeaveRepository: Send + Sync {
    async fn insert(&self, request: &LeaveRequest) -> DomainResult<()>;
    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<LeaveRequest>>;
    async fn find_in_tenant(
        &self,
        tenant: &TenantId,
        id: &Uuid,
    ) -> DomainResult<Option<LeaveRequest>>;
    async fn update_status(&self, request: &LeaveRequest) -> DomainResult<()>;
}
