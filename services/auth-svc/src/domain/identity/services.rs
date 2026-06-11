//! Use-case layer for the identity context.
//!
//! Each public method corresponds to a use-case from AUTH-FLOW.md /
//! SERVICE-auth-svc.md. Handlers call these; these call ports. No HTTP or SQL
//! concepts appear here.

use std::sync::Arc;

use crate::domain::identity::entities::User;
use crate::domain::identity::ports::{
    EventPublisher, IssuedToken, PasswordHasher, RefreshTokenRepository, RoleRepository,
    TenantRepository, TokenIssuer, UserRepository,
};
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::{Clock, Email, TenantId, UserId};

/// Result of a successful login (mapped to `LoginResponse` by the API layer).
#[derive(Debug)]
pub struct LoginOutcome {
    pub access: IssuedToken,
    /// Opaque refresh token (raw value returned once; only its hash is stored).
    pub refresh_token: String,
    pub user: User,
    pub tenant_id: TenantId,
    pub roles: Vec<String>,
}

pub struct IdentityService {
    user_repo: Arc<dyn UserRepository>,
    tenant_repo: Arc<dyn TenantRepository>,
    role_repo: Arc<dyn RoleRepository>,
    refresh_token_repo: Arc<dyn RefreshTokenRepository>,
    token_issuer: Arc<dyn TokenIssuer>,
    password_hasher: Arc<dyn PasswordHasher>,
    events: Arc<dyn EventPublisher>,
    clock: Arc<dyn Clock>,
}

impl IdentityService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        tenant_repo: Arc<dyn TenantRepository>,
        role_repo: Arc<dyn RoleRepository>,
        refresh_token_repo: Arc<dyn RefreshTokenRepository>,
        token_issuer: Arc<dyn TokenIssuer>,
        password_hasher: Arc<dyn PasswordHasher>,
        events: Arc<dyn EventPublisher>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            user_repo,
            tenant_repo,
            role_repo,
            refresh_token_repo,
            token_issuer,
            password_hasher,
            events,
            clock,
        }
    }

    /// AUTH-FLOW.md §4 – Login.
    ///
    /// TODO(Phase 1.2): implement:
    /// 1. find user by email; verify password via `PasswordHasher`;
    /// 2. resolve tenant context (input tenant or user's default);
    /// 3. load roles via `RoleRepository`;
    /// 4. issue access token via `TokenIssuer` + persist hashed refresh token;
    /// 5. return `LoginOutcome`. Log INFO success/fail WITHOUT password/token.
    pub async fn login(
        &self,
        _email: Email,
        _raw_password: String,
        _tenant_id: Option<TenantId>,
    ) -> DomainResult<LoginOutcome> {
        Err(DomainError::Internal(
            "IdentityService::login not implemented yet (Phase 1.2)".into(),
        ))
    }

    /// AUTH-FLOW.md §5 – Refresh.
    ///
    /// TODO(Phase 1.2): validate refresh token (exists, unexpired, unrevoked),
    /// issue a new access token, optionally rotate the refresh token.
    pub async fn refresh(&self, _refresh_token: String) -> DomainResult<LoginOutcome> {
        Err(DomainError::Internal(
            "IdentityService::refresh not implemented yet (Phase 1.2)".into(),
        ))
    }

    /// AUTH-FLOW.md §6 – Logout: revoke the refresh token server-side.
    ///
    /// TODO(Phase 1.2): hash incoming token, call `RefreshTokenRepository::revoke`.
    pub async fn logout(&self, _refresh_token: String) -> DomainResult<()> {
        Err(DomainError::Internal(
            "IdentityService::logout not implemented yet (Phase 1.2)".into(),
        ))
    }

    /// `GET /api/v1/auth/me` – current user profile.
    pub async fn me(&self, user_id: &UserId) -> DomainResult<User> {
        self.user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("user {user_id}")))
    }

    // TODO(Phase 1.3): register_user / create_tenant use-cases. Both must,
    // after the DB commit, publish `UserRegistered` / `TenantCreated` via the
    // `EventPublisher` port (subjects in event-models::auth::subjects).
}
