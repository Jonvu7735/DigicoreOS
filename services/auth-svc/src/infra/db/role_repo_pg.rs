//! `RoleRepository` implementation backed by Postgres
//! (`auth_svc.roles`, `auth_svc.user_roles`, `auth_svc.role_permissions`).

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::identity::entities::Role;
use crate::domain::identity::ports::RoleRepository;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::{RoleId, TenantId, UserId};
use crate::infra::db::map_db_err;

type RoleRow = (Uuid, String, String, Option<String>);

fn to_role(r: RoleRow) -> Role {
    Role {
        id: RoleId(r.0),
        tenant_id: TenantId(r.1),
        name: r.2,
        description: r.3,
    }
}

pub struct PgRoleRepo {
    pool: PgPool,
}

impl PgRoleRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RoleRepository for PgRoleRepo {
    async fn roles_for_user(&self, user: &UserId, tenant: &TenantId) -> DomainResult<Vec<Role>> {
        let rows: Vec<RoleRow> = sqlx::query_as(
            "SELECT r.id, r.tenant_id, r.name, r.description FROM roles r \
             JOIN user_roles ur ON ur.role_id = r.id \
             WHERE ur.user_id = $1 AND r.tenant_id = $2 \
             ORDER BY r.name",
        )
        .bind(user.0)
        .bind(&tenant.0)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(to_role).collect())
    }

    async fn permission_codes_for_role(&self, role: &Role) -> DomainResult<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT permission_code FROM role_permissions WHERE role_id = $1 \
             ORDER BY permission_code",
        )
        .bind(role.id.0)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(|(c,)| c).collect())
    }

    async fn tenant_ids_for_user(&self, user: &UserId) -> DomainResult<Vec<TenantId>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT r.tenant_id FROM roles r \
             JOIN user_roles ur ON ur.role_id = r.id \
             WHERE ur.user_id = $1",
        )
        .bind(user.0)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(|(t,)| TenantId(t)).collect())
    }
}
