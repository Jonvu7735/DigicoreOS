//! Ports for the activities context (implemented in infra/db).
//!
//! Activities have no event contract in EVENTS.md, so the repository is plain
//! CRUD (no outbox).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::activities::entities::Activity;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

#[async_trait]
pub trait ActivityRepository: Send + Sync {
    async fn insert(&self, activity: &Activity) -> DomainResult<()>;
    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Activity>>;
    async fn find_in_tenant(&self, tenant: &TenantId, id: &Uuid) -> DomainResult<Option<Activity>>;
    async fn update(&self, activity: &Activity) -> DomainResult<()>;
}
