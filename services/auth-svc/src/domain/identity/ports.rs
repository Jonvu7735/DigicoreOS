//! Ports (traits) the identity context needs from the outside world.
//!
//! `infra/` provides the implementations; `bootstrap/wiring.rs` binds them.
//! Domain services depend ONLY on these traits.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use platform_outbox::OutboxMessage;

use crate::domain::identity::entities::{RefreshToken, Role, Tenant, User};
use crate::domain::identity::provisioning::{NewRole, TenantProvisioning};
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
    async fn update(&self, tenant: &Tenant) -> DomainResult<()>;
    /// All tenants, newest first (paginated). Platform super-admin listing.
    async fn list(&self, limit: i64, offset: i64) -> DomainResult<Vec<Tenant>>;
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

/// Snapshot of an account's failed-login state (SECURITY.md §5.2: protect
/// `/auth/login` from brute force). `locked_until` in the future means the
/// account is currently locked.
#[derive(Debug, Clone, Default)]
pub struct LoginLockStatus {
    pub failed_count: i64,
    pub locked_until: Option<DateTime<Utc>>,
}

/// Tracks consecutive failed logins per account and locks it after a threshold.
/// Backed by Postgres (`auth_svc.login_attempts`); shared across replicas so a
/// lockout holds platform-wide, not just per-process.
#[async_trait]
pub trait LoginAttemptRepository: Send + Sync {
    /// Current lock state for an email (a past `locked_until` means unlocked).
    async fn status(&self, email: &Email) -> DomainResult<LoginLockStatus>;
    /// Record one failed attempt. When the running count reaches `threshold`,
    /// set `locked_until = now + lock_for` and reset the counter. Returns the
    /// resulting state. Implementations MUST apply this atomically.
    async fn record_failure(
        &self,
        email: &Email,
        now: DateTime<Utc>,
        threshold: i64,
        lock_for: Duration,
    ) -> DomainResult<LoginLockStatus>;
    /// Clear all failed-attempt state for an email (on a successful login).
    async fn reset(&self, email: &Email) -> DomainResult<()>;
}

/// Transactional writes that also enqueue events into the outbox in the SAME
/// transaction (DATA-STRATEGY.md §3.2), so state and events commit together.
/// A duplicate owner email surfaces as `DomainError::Conflict`.
#[async_trait]
pub trait ProvisioningRepository: Send + Sync {
    /// Tenant + owner + default roles/permissions + owner role assignment, plus
    /// `spec.events`, in one transaction.
    async fn provision_tenant(&self, spec: &TenantProvisioning) -> DomainResult<()>;
    /// An owner-less tenant + its default roles/permissions + `events`, in one
    /// transaction. Used by the platform super-admin to create a tenant shell
    /// (users are added afterwards via `provision_user_in_tenant`).
    async fn provision_tenant_shell(
        &self,
        tenant: &Tenant,
        roles: &[NewRole],
        events: &[OutboxMessage],
    ) -> DomainResult<()>;
    /// Create `user`, assign `role_name` within `tenant`, and enqueue `events`,
    /// in one transaction. `NotFound` if the role is absent in the tenant.
    async fn provision_user_in_tenant(
        &self,
        user: &User,
        tenant: &TenantId,
        role_name: &str,
        events: &[OutboxMessage],
    ) -> DomainResult<()>;
    /// Update a user's mutable fields and enqueue `events`, in one transaction.
    async fn update_user(&self, user: &User, events: &[OutboxMessage]) -> DomainResult<()>;
}

// The outbox relay ports (read/clear + raw publisher) and their worker live in
// the shared `platform-outbox` crate (DATA-STRATEGY.md §3.2).

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
