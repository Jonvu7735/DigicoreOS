//! `ProvisioningRepository` backed by Postgres. Creates a tenant, its owner,
//! the default roles + `role_permissions`, and the owner's role assignment in a
//! single transaction so a new tenant never lands half-created.

use async_trait::async_trait;

use sqlx::PgPool;

use crate::domain::identity::ports::ProvisioningRepository;
use crate::domain::identity::provisioning::TenantProvisioning;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::infra::db::{map_db_err, map_write_err};

pub struct PgProvisioningRepo {
    pool: PgPool,
}

impl PgProvisioningRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProvisioningRepository for PgProvisioningRepo {
    async fn provision_tenant(&self, spec: &TenantProvisioning) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;

        sqlx::query(
            "INSERT INTO tenants (id, name, plan, is_active, created_at) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&spec.tenant.id.0)
        .bind(&spec.tenant.name)
        .bind(&spec.tenant.plan)
        .bind(spec.tenant.is_active)
        .bind(spec.tenant.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_write_err)?;

        sqlx::query(
            "INSERT INTO users (id, email, display_name, password_hash, is_active, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(spec.owner.id.0)
        .bind(&spec.owner.email.0)
        .bind(&spec.owner.display_name)
        .bind(&spec.owner.password_hash)
        .bind(spec.owner.is_active)
        .bind(spec.owner.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_write_err)?;

        for role in &spec.roles {
            sqlx::query(
                "INSERT INTO roles (id, tenant_id, name, description) VALUES ($1, $2, $3, $4)",
            )
            .bind(role.id.0)
            .bind(&spec.tenant.id.0)
            .bind(&role.name)
            .bind(&role.description)
            .execute(&mut *tx)
            .await
            .map_err(map_write_err)?;

            for code in &role.permission_codes {
                sqlx::query(
                    "INSERT INTO role_permissions (role_id, permission_code) VALUES ($1, $2)",
                )
                .bind(role.id.0)
                .bind(code)
                .execute(&mut *tx)
                .await
                .map_err(map_write_err)?;
            }
        }

        let owner_role_id = spec
            .roles
            .iter()
            .find(|r| r.name == spec.owner_role)
            .map(|r| r.id.0)
            .ok_or_else(|| {
                DomainError::Internal("owner role missing from provisioning spec".into())
            })?;

        sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)")
            .bind(spec.owner.id.0)
            .bind(owner_role_id)
            .execute(&mut *tx)
            .await
            .map_err(map_write_err)?;

        tx.commit().await.map_err(map_db_err)?;
        Ok(())
    }
}
