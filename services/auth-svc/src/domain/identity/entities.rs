//! Pure domain entities for the identity context.
//!
//! These map to the `auth_svc` schema tables (SERVICE-auth-svc.md §4):
//! `tenants`, `users`, `roles`, `permissions`, `user_roles`,
//! `role_permissions`, `refresh_tokens`. They are NEVER serialized directly to
//! HTTP responses – `api/http/dto` owns the wire shapes.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::shared::types::{Email, PermissionCode, RoleId, TenantId, UserId};

#[derive(Debug, Clone)]
pub struct Tenant {
    pub id: TenantId,
    pub name: String,
    /// Subscription plan identifier (free/pro/enterprise/...).
    pub plan: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub email: Email,
    pub display_name: String,
    /// Argon2 hash – the raw password never leaves the login handler.
    pub password_hash: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Role scoped to a tenant (multi-tenant RBAC, SECURITY.md).
#[derive(Debug, Clone)]
pub struct Role {
    pub id: RoleId,
    pub tenant_id: TenantId,
    /// e.g. `OWNER`, `ADMIN`, `MEMBER` – appears in JWT `roles` claim.
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Permission {
    pub code: PermissionCode,
    pub description: Option<String>,
}

/// Server-side refresh token record (AUTH-FLOW.md §5: validate existence,
/// expiry, revocation; support rotation).
#[derive(Debug, Clone)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: UserId,
    pub tenant_id: TenantId,
    /// Only a hash of the opaque token is stored (SECURITY.md: never store
    /// raw tokens).
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
