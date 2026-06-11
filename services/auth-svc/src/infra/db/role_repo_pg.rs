//! `RoleRepository` implementation backed by Postgres
//! (`auth_svc.roles`, `auth_svc.user_roles`, `auth_svc.role_permissions`).

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::identity::entities::Role;
use crate::domain::identity::ports::RoleRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::{TenantId, UserId};

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
    // TODO(Phase 1.2): SELECT r.* FROM roles r
    //   JOIN user_roles ur ON ur.role_id = r.id
    //   WHERE ur.user_id = $1 AND r.tenant_id = $2
    async fn roles_for_user(
        &self,
        _user: &UserId,
        _tenant: &TenantId,
    ) -> DomainResult<Vec<Role>> {
        Err(DomainError::Internal(
            "PgRoleRepo::roles_for_user not implemented (Phase 1.2)".into(),
        ))
    }

    // TODO(Phase 1.3): SELECT p.code FROM permissions p
    //   JOIN role_permissions rp ON rp.permission_code = p.code
    //   WHERE rp.role_id = $1
    async fn permission_codes_for_role(&self, _role: &Role) -> DomainResult<Vec<String>> {
        Err(DomainError::Internal(
            "PgRoleRepo::permission_codes_for_role not implemented (Phase 1.3)".into(),
        ))
    }
}
