//! Ports for the contacts context (implemented in infra/db).
//!
//! Contacts have no event contract in EVENTS.md, so the repository is plain CRUD
//! (no outbox).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::contacts::entities::Contact;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

#[async_trait]
pub trait ContactRepository: Send + Sync {
    async fn insert(&self, contact: &Contact) -> DomainResult<()>;
    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Contact>>;
    async fn find_in_tenant(&self, tenant: &TenantId, id: &Uuid) -> DomainResult<Option<Contact>>;
    async fn update(&self, contact: &Contact) -> DomainResult<()>;
}
