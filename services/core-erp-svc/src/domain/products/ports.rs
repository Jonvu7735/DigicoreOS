//! Ports for the products context (implemented in infra/db).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::products::entities::Product;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

#[async_trait]
pub trait ProductRepository: Send + Sync {
    async fn insert(&self, product: &Product) -> DomainResult<()>;
    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Product>>;
    async fn find_in_tenant(&self, tenant: &TenantId, id: &Uuid) -> DomainResult<Option<Product>>;
    async fn update(&self, product: &Product) -> DomainResult<()>;
}
