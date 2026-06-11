//! `TenantRepository` implementation backed by Postgres (`auth_svc.tenants`).

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::identity::entities::Tenant;
use crate::domain::identity::ports::TenantRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::TenantId;

pub struct PgTenantRepo {
    pool: PgPool,
}

impl PgTenantRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TenantRepository for PgTenantRepo {
    // TODO(Phase 1.3): SELECT ... FROM tenants WHERE id = $1
    async fn find_by_id(&self, _id: &TenantId) -> DomainResult<Option<Tenant>> {
        Err(DomainError::Internal(
            "PgTenantRepo::find_by_id not implemented (Phase 1.3)".into(),
        ))
    }

    // TODO(Phase 1.3): INSERT INTO tenants (...) VALUES (...). Creating a
    // tenant must also write a `TenantCreated` row to the outbox in the SAME
    // transaction (DATA-STRATEGY.md §3.2).
    async fn insert(&self, _tenant: &Tenant) -> DomainResult<()> {
        Err(DomainError::Internal(
            "PgTenantRepo::insert not implemented (Phase 1.3)".into(),
        ))
    }
}
