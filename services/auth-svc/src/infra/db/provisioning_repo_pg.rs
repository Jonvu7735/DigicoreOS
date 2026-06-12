//! `ProvisioningRepository` backed by Postgres. State changes and their outbox
//! events are written in ONE transaction (DATA-STRATEGY.md §3.2), so a new
//! tenant/user and its events always commit together.

use async_trait::async_trait;
use platform_outbox::{insert_outbox, OutboxMessage};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::identity::entities::{Tenant, User};
use crate::domain::identity::ports::ProvisioningRepository;
use crate::domain::identity::provisioning::{NewRole, TenantProvisioning};
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::TenantId;
use crate::infra::db::{map_db_err, map_write_err};

/// Map a shared-outbox storage error into this service's `DomainError`.
fn outbox_err(e: platform_outbox::OutboxError) -> DomainError {
    DomainError::Internal(e.to_string())
}

pub struct PgProvisioningRepo {
    pool: PgPool,
}

impl PgProvisioningRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

async fn insert_user(conn: &mut sqlx::PgConnection, user: &User) -> DomainResult<()> {
    sqlx::query(
        "INSERT INTO users (id, email, display_name, password_hash, is_active, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(user.id.0)
    .bind(&user.email.0)
    .bind(&user.display_name)
    .bind(&user.password_hash)
    .bind(user.is_active)
    .bind(user.created_at)
    .execute(conn)
    .await
    .map_err(map_write_err)?;
    Ok(())
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

        insert_user(&mut tx, &spec.owner).await?;

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

        for event in &spec.events {
            insert_outbox(&mut tx, event).await.map_err(outbox_err)?;
        }

        tx.commit().await.map_err(map_db_err)?;
        Ok(())
    }

    async fn provision_tenant_shell(
        &self,
        tenant: &Tenant,
        roles: &[NewRole],
        events: &[OutboxMessage],
    ) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;

        sqlx::query(
            "INSERT INTO tenants (id, name, plan, is_active, created_at) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&tenant.id.0)
        .bind(&tenant.name)
        .bind(&tenant.plan)
        .bind(tenant.is_active)
        .bind(tenant.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_write_err)?;

        for role in roles {
            sqlx::query(
                "INSERT INTO roles (id, tenant_id, name, description) VALUES ($1, $2, $3, $4)",
            )
            .bind(role.id.0)
            .bind(&tenant.id.0)
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

        for event in events {
            insert_outbox(&mut tx, event).await.map_err(outbox_err)?;
        }

        tx.commit().await.map_err(map_db_err)?;
        Ok(())
    }

    async fn provision_user_in_tenant(
        &self,
        user: &User,
        tenant: &TenantId,
        role_name: &str,
        events: &[OutboxMessage],
    ) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;

        let role: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM roles WHERE tenant_id = $1 AND name = $2")
                .bind(&tenant.0)
                .bind(role_name)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_err)?;
        let role_id = role
            .ok_or_else(|| {
                DomainError::NotFound(format!("role {role_name} in tenant {}", tenant.0))
            })?
            .0;

        insert_user(&mut tx, user).await?;

        sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)")
            .bind(user.id.0)
            .bind(role_id)
            .execute(&mut *tx)
            .await
            .map_err(map_write_err)?;

        for event in events {
            insert_outbox(&mut tx, event).await.map_err(outbox_err)?;
        }

        tx.commit().await.map_err(map_db_err)?;
        Ok(())
    }

    async fn update_user(&self, user: &User, events: &[OutboxMessage]) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;

        sqlx::query(
            "UPDATE users SET email = $2, display_name = $3, password_hash = $4, is_active = $5 \
             WHERE id = $1",
        )
        .bind(user.id.0)
        .bind(&user.email.0)
        .bind(&user.display_name)
        .bind(&user.password_hash)
        .bind(user.is_active)
        .execute(&mut *tx)
        .await
        .map_err(map_write_err)?;

        for event in events {
            insert_outbox(&mut tx, event).await.map_err(outbox_err)?;
        }

        tx.commit().await.map_err(map_db_err)?;
        Ok(())
    }
}
