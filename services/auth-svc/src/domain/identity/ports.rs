//! Ports (traits) the identity context needs from the outside world.
//!
//! `infra/` provides the implementations; `bootstrap/wiring.rs` binds them.
//! Domain services depend ONLY on these traits.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use event_models::auth::AuthEvent;

use crate::domain::identity::entities::{RefreshToken, Role, Tenant, User};
use crate::domain::identity::provisioning::TenantProvisioning;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::{Email, TenantId, UserId};

// ---------------------------------------------------------------------------
// Persistence ports (implemented in infra/db/*_repo_pg.rs, schema `auth_svc`)
// ---------------------------------------------------------------------------

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: &UserId) -> DomainResult<Option<User>>;
    async fn find_by_email(&self, email: &Email) -> DomainResult<Option<User>>;
    async fn insert(&self, user: &User) -> DomainResult<()>;
    async fn update(&self, user: &User) -> DomainResult<()>;
    /// Users holding any role in `tenant` (paginated, newest first). Backs admin
    /// listing with tenant isolation.
    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<User>>;
    /// A user that holds any role in `tenant` (tenant isolation for admin reads).
    async fn find_in_tenant(&self, tenant: &TenantId, id: &UserId) -> DomainResult<Option<User>>;
}

#[async_trait]
pub trait TenantRepository: Send + Sync {
    async fn find_by_id(&self, id: &TenantId) -> DomainResult<Option<Tenant>>;
    async fn insert(&self, tenant: &Tenant) -> DomainResult<()>;
}

#[async_trait]
pub trait RoleRepository: Send + Sync {
    /// Roles of a user within one tenant (feeds the JWT `roles` claim).
    async fn roles_for_user(&self, user: &UserId, tenant: &TenantId) -> DomainResult<Vec<Role>>;
    /// Permission codes granted to a role (SECURITY.md role -> permission map).
    async fn permission_codes_for_role(&self, role: &Role) -> DomainResult<Vec<String>>;
    /// Distinct tenants a user has any role in. Used to infer the login tenant
    /// when the client omits `tenant_id` and the user belongs to exactly one.
    async fn tenant_ids_for_user(&self, user: &UserId) -> DomainResult<Vec<TenantId>>;
}

#[async_trait]
pub trait RefreshTokenRepository: Send + Sync {
    async fn insert(&self, token: &RefreshToken) -> DomainResult<()>;
    /// Look up a non-expired, non-revoked token by its hash.
    async fn find_valid_by_hash(&self, token_hash: &str) -> DomainResult<Option<RefreshToken>>;
    async fn revoke(&self, token_hash: &str) -> DomainResult<()>;
}

/// Atomic tenant provisioning: persists the tenant, its owner, default roles +
/// `role_permissions`, and the owner's role assignment in ONE transaction.
/// A duplicate owner email surfaces as `DomainError::Conflict`.
#[async_trait]
pub trait ProvisioningRepository: Send + Sync {
    async fn provision_tenant(&self, spec: &TenantProvisioning) -> DomainResult<()>;
    /// Create `user` and assign them `role_name` within `tenant`, atomically.
    /// `NotFound` if the role does not exist in the tenant; `Conflict` on a
    /// duplicate email.
    async fn provision_user_in_tenant(
        &self,
        user: &User,
        tenant: &TenantId,
        role_name: &str,
    ) -> DomainResult<()>;
}

// ---------------------------------------------------------------------------
// Security ports (implemented in infra/security/)
// ---------------------------------------------------------------------------

/// Password hashing/verification (Argon2 in infra). Named to mirror the
/// concept, not the algorithm, so the algorithm can be swapped.
pub trait PasswordHasher: Send + Sync {
    fn hash(&self, raw_password: &str) -> DomainResult<String>;
    fn verify(&self, raw_password: &str, password_hash: &str) -> DomainResult<bool>;
}

/// Claims carried by an access token (AUTH-FLOW.md §3).
#[derive(Debug, Clone)]
pub struct AccessTokenClaims {
    /// Subject – user id.
    pub sub: String,
    pub tenant_id: String,
    pub roles: Vec<String>,
    pub iat: i64,
    pub exp: i64,
    pub iss: String,
    pub aud: String,
}

/// A freshly issued access token plus its TTL (for the HTTP response body).
#[derive(Debug, Clone)]
pub struct IssuedToken {
    pub token: String,
    pub expires_in: i64,
}

/// Opaque refresh-token generation + hashing (implemented in
/// infra/security/refresh_token.rs). The raw value is returned to the client
/// once; only its hash is persisted (SECURITY.md: never store raw tokens).
pub trait RefreshTokenHasher: Send + Sync {
    /// A fresh, high-entropy opaque token (the raw value handed to the client).
    fn generate_opaque(&self) -> String;
    /// Deterministic hash of a raw token, used for storage and lookup.
    fn hash(&self, raw: &str) -> String;
}

/// JWT issue/validate port (implemented by infra/security/jwt.rs).
pub trait TokenIssuer: Send + Sync {
    fn issue_access_token(
        &self,
        user_id: &UserId,
        tenant_id: &TenantId,
        roles: &[String],
        now: DateTime<Utc>,
    ) -> DomainResult<IssuedToken>;

    fn validate_access_token(&self, token: &str) -> DomainResult<AccessTokenClaims>;
}

// ---------------------------------------------------------------------------
// Messaging port (implemented in infra/messaging/nats.rs)
// ---------------------------------------------------------------------------

/// Business-event publisher (EVENTS.md).
///
/// Contract: events are published AFTER the owning DB transaction commits.
/// TODO(Phase 1.5): route publishes through an outbox table inside the same
/// transaction + a relay worker (DATA-STRATEGY.md §3.2) instead of direct
/// post-commit publishing.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, event: AuthEvent) -> DomainResult<()>;
}
