//! `TenantRepository` implementation backed by Postgres (`auth_svc.tenants`).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::identity::entities::Tenant;
use crate::domain::identity::ports::TenantRepository;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;
use crate::infra::db::{map_db_err, map_write_err};

type TenantRow = (String, String, String, bool, DateTime<Utc>);

fn to_tenant(r: TenantRow) -> Tenant {
    Tenant {
        id: TenantId(r.0),
        name: r.1,
        plan: r.2,
        is_active: r.3,
        created_at: r.4,
    }
}

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
    async fn find_by_id(&self, id: &TenantId) -> DomainResult<Option<Tenant>> {
        let row: Option<TenantRow> = sqlx::query_as(
            "SELECT id, name, plan, is_active, created_at FROM tenants WHERE id = $1",
        )
        .bind(&id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(row.map(to_tenant))
    }

    async fn insert(&self, tenant: &Tenant) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO tenants (id, name, plan, is_active, created_at) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&tenant.id.0)
        .bind(&tenant.name)
        .bind(&tenant.plan)
        .bind(tenant.is_active)
        .bind(tenant.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_write_err)?;
        Ok(())
    }

    async fn update(&self, tenant: &Tenant) -> DomainResult<()> {
        // `created_at` is immutable; the caller has already verified existence.
        sqlx::query("UPDATE tenants SET name = $2, plan = $3, is_active = $4 WHERE id = $1")
            .bind(&tenant.id.0)
            .bind(&tenant.name)
            .bind(&tenant.plan)
            .bind(tenant.is_active)
            .execute(&self.pool)
            .await
            .map_err(map_write_err)?;
        Ok(())
    }

    async fn list(&self, limit: i64, offset: i64) -> DomainResult<Vec<Tenant>> {
        let rows: Vec<TenantRow> = sqlx::query_as(
            "SELECT id, name, plan, is_active, created_at FROM tenants \
             ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(to_tenant).collect())
    }
}
