//! Use-case layer for the identity context.
//!
//! Each public method corresponds to a use-case from AUTH-FLOW.md /
//! SERVICE-auth-svc.md. Handlers call these; these call ports. No HTTP, SQL, or
//! logging/metrics concepts appear here (those live in the api/infra layers).

use std::sync::Arc;

use chrono::Duration;
use event_models::auth::{AuthEvent, TenantCreated, UserRegistered, UserUpdated};
use event_models::EventHeader;
use uuid::Uuid;

use crate::domain::identity::entities::{RefreshToken, Tenant, User};
use crate::domain::identity::outbox::OutboxMessage;
use crate::domain::identity::ports::{
    AccessTokenClaims, IssuedToken, PasswordHasher, ProvisioningRepository, RefreshTokenHasher,
    RefreshTokenRepository, RoleRepository, TenantRepository, TokenIssuer, UserRepository,
};
use crate::domain::identity::provisioning::{NewRole, TenantProvisioning};
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::{Clock, Email, RoleId, TenantId, UserId};
use platform_auth::rbac;

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

/// A user plus their role names within a tenant (admin read model).
#[derive(Debug)]
pub struct UserView {
    pub user: User,
    pub roles: Vec<String>,
}

pub struct IdentityService {
    user_repo: Arc<dyn UserRepository>,
    tenant_repo: Arc<dyn TenantRepository>,
    role_repo: Arc<dyn RoleRepository>,
    refresh_token_repo: Arc<dyn RefreshTokenRepository>,
    provisioning: Arc<dyn ProvisioningRepository>,
    token_issuer: Arc<dyn TokenIssuer>,
    password_hasher: Arc<dyn PasswordHasher>,
    refresh_token_hasher: Arc<dyn RefreshTokenHasher>,
    clock: Arc<dyn Clock>,
    refresh_ttl_secs: i64,
}

impl IdentityService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        tenant_repo: Arc<dyn TenantRepository>,
        role_repo: Arc<dyn RoleRepository>,
        refresh_token_repo: Arc<dyn RefreshTokenRepository>,
        provisioning: Arc<dyn ProvisioningRepository>,
        token_issuer: Arc<dyn TokenIssuer>,
        password_hasher: Arc<dyn PasswordHasher>,
        refresh_token_hasher: Arc<dyn RefreshTokenHasher>,
        clock: Arc<dyn Clock>,
        refresh_ttl_secs: i64,
    ) -> Self {
        Self {
            user_repo,
            tenant_repo,
            role_repo,
            refresh_token_repo,
            provisioning,
            token_issuer,
            password_hasher,
            refresh_token_hasher,
            clock,
            refresh_ttl_secs,
        }
    }

    /// AUTH-FLOW.md §4 – Login.
    ///
    /// Verify credentials, resolve the tenant context, load roles, then issue an
    /// access token and a stored (hashed) refresh token. Returns `Unauthorized`
    /// for any credential failure without revealing which part failed.
    pub async fn login(
        &self,
        email: Email,
        raw_password: String,
        tenant_id: Option<TenantId>,
    ) -> DomainResult<LoginOutcome> {
        let user = self
            .user_repo
            .find_by_email(&email)
            .await?
            .ok_or_else(|| DomainError::Unauthorized("invalid email or password".into()))?;

        if !user.is_active {
            return Err(DomainError::Unauthorized("account is inactive".into()));
        }
        if !self
            .password_hasher
            .verify(&raw_password, &user.password_hash)?
        {
            return Err(DomainError::Unauthorized(
                "invalid email or password".into(),
            ));
        }

        let tenant_id = self.resolve_login_tenant(&user.id, tenant_id).await?;
        let roles = self.role_names(&user.id, &tenant_id).await?;
        self.issue_session(user, tenant_id, roles).await
    }

    /// AUTH-FLOW.md §5 – Refresh (with rotation: the presented token is revoked
    /// and a new one issued).
    pub async fn refresh(&self, raw_refresh_token: String) -> DomainResult<LoginOutcome> {
        let hash = self.refresh_token_hasher.hash(&raw_refresh_token);
        let existing = self
            .refresh_token_repo
            .find_valid_by_hash(&hash)
            .await?
            .ok_or_else(|| DomainError::Unauthorized("invalid or expired refresh token".into()))?;

        let user = self
            .user_repo
            .find_by_id(&existing.user_id)
            .await?
            .ok_or_else(|| DomainError::Unauthorized("user no longer exists".into()))?;
        if !user.is_active {
            return Err(DomainError::Unauthorized("account is inactive".into()));
        }

        let tenant_id = existing.tenant_id.clone();
        let roles = self.role_names(&user.id, &tenant_id).await?;

        // Rotate: revoke the presented token before issuing a fresh session.
        self.refresh_token_repo.revoke(&existing.token_hash).await?;
        self.issue_session(user, tenant_id, roles).await
    }

    /// AUTH-FLOW.md §6 – Logout: revoke the refresh token server-side.
    /// Idempotent – an unknown token is treated as already logged out.
    pub async fn logout(&self, raw_refresh_token: String) -> DomainResult<()> {
        let hash = self.refresh_token_hasher.hash(&raw_refresh_token);
        self.refresh_token_repo.revoke(&hash).await
    }

    /// `GET /api/v1/auth/me` – current user profile.
    pub async fn me(&self, user_id: &UserId) -> DomainResult<User> {
        self.user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("user {user_id}")))
    }

    /// Verify an access token (used by the auth middleware – AUTH-FLOW.md §7).
    /// Delegates to the `TokenIssuer` port; no DB access.
    pub fn validate_access_token(&self, token: &str) -> DomainResult<AccessTokenClaims> {
        self.token_issuer.validate_access_token(token)
    }

    /// Self-serve sign-up (API-GATEWAY.md §2.2): create a new tenant with its
    /// owner user and the default RBAC roles atomically, publish
    /// `TenantCreated` + `UserRegistered`, then return a session for the owner
    /// (sign-up doubles as the owner's first login).
    pub async fn register(
        &self,
        tenant_name: String,
        plan: Option<String>,
        email: Email,
        raw_password: String,
        display_name: String,
    ) -> DomainResult<LoginOutcome> {
        let tenant_name = tenant_name.trim().to_string();
        let display_name = display_name.trim().to_string();
        if tenant_name.is_empty() {
            return Err(DomainError::Validation("tenant_name is required".into()));
        }
        if display_name.is_empty() {
            return Err(DomainError::Validation("display_name is required".into()));
        }
        if !email.0.contains('@') {
            return Err(DomainError::Validation("a valid email is required".into()));
        }
        if raw_password.len() < 8 {
            return Err(DomainError::Validation(
                "password must be at least 8 characters".into(),
            ));
        }

        let now = self.clock.now_utc();
        let password_hash = self.password_hasher.hash(&raw_password)?;
        let tenant = Tenant {
            id: TenantId(Uuid::now_v7().to_string()),
            name: tenant_name,
            plan: plan.unwrap_or_else(|| "free".to_string()),
            is_active: true,
            created_at: now,
        };
        let owner = User {
            id: UserId(Uuid::now_v7()),
            email,
            display_name,
            password_hash,
            is_active: true,
            created_at: now,
        };

        let roles = rbac::DEFAULT_ROLES
            .iter()
            .map(|name| NewRole {
                id: RoleId(Uuid::now_v7()),
                name: (*name).to_string(),
                description: Some(rbac::role_description(name).to_string()),
                permission_codes: rbac::permissions_for(name)
                    .into_iter()
                    .map(String::from)
                    .collect(),
            })
            .collect();

        // Events are enqueued into the outbox in the SAME transaction as state.
        let events = vec![
            OutboxMessage::from_auth_event(&self.tenant_created_event(&tenant))?,
            OutboxMessage::from_auth_event(&self.user_registered_event(&tenant.id, &owner))?,
        ];
        let spec = TenantProvisioning {
            tenant: tenant.clone(),
            owner: owner.clone(),
            roles,
            owner_role: "OWNER".to_string(),
            events,
        };
        self.provisioning.provision_tenant(&spec).await?;

        let tenant_id = tenant.id;
        self.issue_session(owner, tenant_id, vec!["OWNER".to_string()])
            .await
    }

    /// Admin: create a user in `tenant_id` with one of the default roles, then
    /// publish `UserRegistered`.
    pub async fn create_user(
        &self,
        tenant_id: &TenantId,
        email: Email,
        raw_password: String,
        display_name: String,
        role: String,
    ) -> DomainResult<UserView> {
        let display_name = display_name.trim().to_string();
        if display_name.is_empty() {
            return Err(DomainError::Validation("display_name is required".into()));
        }
        if !email.0.contains('@') {
            return Err(DomainError::Validation("a valid email is required".into()));
        }
        if raw_password.len() < 8 {
            return Err(DomainError::Validation(
                "password must be at least 8 characters".into(),
            ));
        }
        if !rbac::DEFAULT_ROLES.contains(&role.as_str()) {
            return Err(DomainError::Validation(format!("unknown role: {role}")));
        }

        let user = User {
            id: UserId(Uuid::now_v7()),
            email,
            display_name,
            password_hash: self.password_hasher.hash(&raw_password)?,
            is_active: true,
            created_at: self.clock.now_utc(),
        };
        let events = [OutboxMessage::from_auth_event(
            &self.user_registered_event(tenant_id, &user),
        )?];
        self.provisioning
            .provision_user_in_tenant(&user, tenant_id, &role, &events)
            .await?;

        Ok(UserView {
            user,
            roles: vec![role],
        })
    }

    /// Admin: list users in `tenant_id`.
    pub async fn list_users(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<UserView>> {
        let users = self
            .user_repo
            .list_in_tenant(tenant_id, limit, offset)
            .await?;
        let mut views = Vec::with_capacity(users.len());
        for user in users {
            let roles = self.role_names(&user.id, tenant_id).await?;
            views.push(UserView { user, roles });
        }
        Ok(views)
    }

    /// Admin: fetch one user scoped to `tenant_id`.
    pub async fn get_user(&self, tenant_id: &TenantId, user_id: &UserId) -> DomainResult<UserView> {
        let user = self
            .user_repo
            .find_in_tenant(tenant_id, user_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("user {user_id}")))?;
        let roles = self.role_names(user_id, tenant_id).await?;
        Ok(UserView { user, roles })
    }

    /// Admin: update a user's display name and/or active flag, then publish
    /// `UserUpdated`.
    pub async fn update_user(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        display_name: Option<String>,
        is_active: Option<bool>,
    ) -> DomainResult<UserView> {
        let mut user = self
            .user_repo
            .find_in_tenant(tenant_id, user_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("user {user_id}")))?;

        if let Some(name) = display_name {
            let name = name.trim().to_string();
            if name.is_empty() {
                return Err(DomainError::Validation(
                    "display_name cannot be empty".into(),
                ));
            }
            user.display_name = name;
        }
        if let Some(active) = is_active {
            user.is_active = active;
        }

        let events = [OutboxMessage::from_auth_event(
            &self.user_updated_event(tenant_id, &user),
        )?];
        self.provisioning.update_user(&user, &events).await?;
        let roles = self.role_names(user_id, tenant_id).await?;
        Ok(UserView { user, roles })
    }

    /// Admin: read a tenant (auth_tenant_read; handler enforces own-tenant scope).
    pub async fn get_tenant(&self, tenant_id: &TenantId) -> DomainResult<Tenant> {
        self.tenant_repo
            .find_by_id(tenant_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("tenant {tenant_id}")))
    }

    /// Admin: update a tenant's name / plan / active flag (auth_tenant_update_plan).
    pub async fn update_tenant(
        &self,
        tenant_id: &TenantId,
        name: Option<String>,
        plan: Option<String>,
        is_active: Option<bool>,
    ) -> DomainResult<Tenant> {
        let mut tenant = self.get_tenant(tenant_id).await?;
        if let Some(name) = name {
            let name = name.trim().to_string();
            if name.is_empty() {
                return Err(DomainError::Validation("name cannot be empty".into()));
            }
            tenant.name = name;
        }
        if let Some(plan) = plan {
            tenant.plan = plan;
        }
        if let Some(active) = is_active {
            tenant.is_active = active;
        }
        self.tenant_repo.update(&tenant).await?;
        Ok(tenant)
    }

    // --- internals -----------------------------------------------------------

    /// Issue an access token + a fresh stored refresh token for `(user, tenant)`.
    async fn issue_session(
        &self,
        user: User,
        tenant_id: TenantId,
        roles: Vec<String>,
    ) -> DomainResult<LoginOutcome> {
        let now = self.clock.now_utc();
        let access = self
            .token_issuer
            .issue_access_token(&user.id, &tenant_id, &roles, now)?;

        let raw_refresh = self.refresh_token_hasher.generate_opaque();
        let refresh = RefreshToken {
            id: Uuid::now_v7(),
            user_id: user.id,
            tenant_id: tenant_id.clone(),
            token_hash: self.refresh_token_hasher.hash(&raw_refresh),
            expires_at: now + Duration::seconds(self.refresh_ttl_secs),
            revoked_at: None,
            created_at: now,
        };
        self.refresh_token_repo.insert(&refresh).await?;

        Ok(LoginOutcome {
            access,
            refresh_token: raw_refresh,
            user,
            tenant_id,
            roles,
        })
    }

    async fn role_names(
        &self,
        user_id: &UserId,
        tenant_id: &TenantId,
    ) -> DomainResult<Vec<String>> {
        Ok(self
            .role_repo
            .roles_for_user(user_id, tenant_id)
            .await?
            .into_iter()
            .map(|r| r.name)
            .collect())
    }

    /// Pick the tenant for the session: the requested one, or the user's only
    /// tenant. Ambiguous (multiple) or empty membership is an error.
    async fn resolve_login_tenant(
        &self,
        user_id: &UserId,
        requested: Option<TenantId>,
    ) -> DomainResult<TenantId> {
        if let Some(tenant_id) = requested {
            return Ok(tenant_id);
        }
        let mut tenants = self.role_repo.tenant_ids_for_user(user_id).await?;
        match tenants.len() {
            1 => Ok(tenants.pop().expect("length checked")),
            0 => Err(DomainError::Unauthorized(
                "user does not belong to any tenant".into(),
            )),
            _ => Err(DomainError::Validation(
                "tenant_id is required: user belongs to multiple tenants".into(),
            )),
        }
    }

    fn tenant_created_event(&self, tenant: &Tenant) -> AuthEvent {
        let header = EventHeader::new(
            Uuid::now_v7(),
            self.clock.now_utc(),
            tenant.id.0.clone(),
            "tenant",
            tenant.id.0.clone(),
            "TenantCreated",
            1,
        );
        AuthEvent::TenantCreated(TenantCreated {
            header,
            tenant_id: tenant.id.0.clone(),
            tenant_name: tenant.name.clone(),
            plan: tenant.plan.clone(),
        })
    }

    fn user_registered_event(&self, tenant_id: &TenantId, user: &User) -> AuthEvent {
        let header = EventHeader::new(
            Uuid::now_v7(),
            self.clock.now_utc(),
            tenant_id.0.clone(),
            "user",
            user.id.to_string(),
            "UserRegistered",
            1,
        );
        AuthEvent::UserRegistered(UserRegistered {
            header,
            user_id: user.id.to_string(),
            email: user.email.0.clone(),
            display_name: user.display_name.clone(),
            is_active: user.is_active,
        })
    }

    fn user_updated_event(&self, tenant_id: &TenantId, user: &User) -> AuthEvent {
        let header = EventHeader::new(
            Uuid::now_v7(),
            self.clock.now_utc(),
            tenant_id.0.clone(),
            "user",
            user.id.to_string(),
            "UserUpdated",
            1,
        );
        AuthEvent::UserUpdated(UserUpdated {
            header,
            user_id: user.id.to_string(),
            email: user.email.0.clone(),
            display_name: user.display_name.clone(),
            is_active: user.is_active,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    use super::IdentityService;
    use crate::domain::identity::entities::{RefreshToken, Role, Tenant, User};
    use crate::domain::identity::outbox::OutboxMessage;
    use crate::domain::identity::ports::{
        AccessTokenClaims, IssuedToken, PasswordHasher, ProvisioningRepository, RefreshTokenHasher,
        RefreshTokenRepository, RoleRepository, TenantRepository, TokenIssuer, UserRepository,
    };
    use crate::domain::identity::provisioning::TenantProvisioning;
    use crate::domain::shared::error::{DomainError, DomainResult};
    use crate::domain::shared::types::{Clock, Email, RoleId, TenantId, UserId};
    use platform_auth::rbac;

    // --- fake ports (no DB / crypto needed) ---------------------------------

    struct FakeUserRepo {
        users: Vec<User>,
    }
    #[async_trait]
    impl UserRepository for FakeUserRepo {
        async fn find_by_id(&self, id: &UserId) -> DomainResult<Option<User>> {
            Ok(self.users.iter().find(|u| u.id == *id).cloned())
        }
        async fn find_by_email(&self, email: &Email) -> DomainResult<Option<User>> {
            Ok(self
                .users
                .iter()
                .find(|u| u.email.0.eq_ignore_ascii_case(&email.0))
                .cloned())
        }
        async fn insert(&self, _user: &User) -> DomainResult<()> {
            Ok(())
        }
        async fn update(&self, _user: &User) -> DomainResult<()> {
            Ok(())
        }
        async fn list_in_tenant(
            &self,
            _tenant: &TenantId,
            _limit: i64,
            _offset: i64,
        ) -> DomainResult<Vec<User>> {
            Ok(self.users.clone())
        }
        async fn find_in_tenant(
            &self,
            _tenant: &TenantId,
            id: &UserId,
        ) -> DomainResult<Option<User>> {
            Ok(self.users.iter().find(|u| u.id == *id).cloned())
        }
    }

    struct FakeTenantRepo;
    #[async_trait]
    impl TenantRepository for FakeTenantRepo {
        async fn find_by_id(&self, _id: &TenantId) -> DomainResult<Option<Tenant>> {
            Ok(None)
        }
        async fn insert(&self, _tenant: &Tenant) -> DomainResult<()> {
            Ok(())
        }
        async fn update(&self, _tenant: &Tenant) -> DomainResult<()> {
            Ok(())
        }
    }

    /// Tenant repo that holds one tenant, for tenant get/update tests.
    #[derive(Default)]
    struct RecordingTenantRepo {
        tenant: Mutex<Option<Tenant>>,
    }
    #[async_trait]
    impl TenantRepository for RecordingTenantRepo {
        async fn find_by_id(&self, _id: &TenantId) -> DomainResult<Option<Tenant>> {
            Ok(self.tenant.lock().unwrap().clone())
        }
        async fn insert(&self, tenant: &Tenant) -> DomainResult<()> {
            *self.tenant.lock().unwrap() = Some(tenant.clone());
            Ok(())
        }
        async fn update(&self, tenant: &Tenant) -> DomainResult<()> {
            *self.tenant.lock().unwrap() = Some(tenant.clone());
            Ok(())
        }
    }

    struct FakeRoleRepo {
        tenants: Vec<TenantId>,
        roles: Vec<Role>,
    }
    #[async_trait]
    impl RoleRepository for FakeRoleRepo {
        async fn roles_for_user(&self, _u: &UserId, _t: &TenantId) -> DomainResult<Vec<Role>> {
            Ok(self.roles.clone())
        }
        async fn permission_codes_for_role(&self, _role: &Role) -> DomainResult<Vec<String>> {
            Ok(vec![])
        }
        async fn tenant_ids_for_user(&self, _u: &UserId) -> DomainResult<Vec<TenantId>> {
            Ok(self.tenants.clone())
        }
    }

    #[derive(Default)]
    struct FakeRefreshRepo {
        inserted: Mutex<Vec<RefreshToken>>,
        revoked: Mutex<Vec<String>>,
    }
    #[async_trait]
    impl RefreshTokenRepository for FakeRefreshRepo {
        async fn insert(&self, token: &RefreshToken) -> DomainResult<()> {
            self.inserted.lock().unwrap().push(token.clone());
            Ok(())
        }
        async fn find_valid_by_hash(&self, hash: &str) -> DomainResult<Option<RefreshToken>> {
            let revoked = self.revoked.lock().unwrap();
            Ok(self
                .inserted
                .lock()
                .unwrap()
                .iter()
                .find(|t| t.token_hash == hash && !revoked.contains(&t.token_hash))
                .cloned())
        }
        async fn revoke(&self, hash: &str) -> DomainResult<()> {
            self.revoked.lock().unwrap().push(hash.to_string());
            Ok(())
        }
    }

    struct FakeTokenIssuer;
    impl TokenIssuer for FakeTokenIssuer {
        fn issue_access_token(
            &self,
            _u: &UserId,
            _t: &TenantId,
            roles: &[String],
            _now: DateTime<Utc>,
        ) -> DomainResult<IssuedToken> {
            Ok(IssuedToken {
                token: format!("access[{}]", roles.join(",")),
                expires_in: 1800,
            })
        }
        fn validate_access_token(&self, _token: &str) -> DomainResult<AccessTokenClaims> {
            unimplemented!("not exercised by these tests")
        }
    }

    struct FakePasswordHasher {
        ok: bool,
    }
    impl PasswordHasher for FakePasswordHasher {
        fn hash(&self, _raw: &str) -> DomainResult<String> {
            Ok("hash".into())
        }
        fn verify(&self, _raw: &str, _hash: &str) -> DomainResult<bool> {
            Ok(self.ok)
        }
    }

    #[derive(Default)]
    struct FakeRefreshHasher {
        counter: AtomicU64,
    }
    impl RefreshTokenHasher for FakeRefreshHasher {
        fn generate_opaque(&self) -> String {
            format!("raw-{}", self.counter.fetch_add(1, Ordering::SeqCst))
        }
        fn hash(&self, raw: &str) -> String {
            format!("h:{raw}")
        }
    }

    /// Captures provisioning specs/users and all enqueued event type names so
    /// tests can assert on what would be written to the outbox.
    #[derive(Default)]
    struct FakeProvisioningRepo {
        specs: Mutex<Vec<TenantProvisioning>>,
        users: Mutex<Vec<(User, TenantId, String)>>,
        events: Mutex<Vec<String>>,
    }
    impl FakeProvisioningRepo {
        fn record(&self, events: &[OutboxMessage]) {
            self.events
                .lock()
                .unwrap()
                .extend(events.iter().map(|e| e.event_type.clone()));
        }
    }
    #[async_trait]
    impl ProvisioningRepository for FakeProvisioningRepo {
        async fn provision_tenant(&self, spec: &TenantProvisioning) -> DomainResult<()> {
            self.record(&spec.events);
            self.specs.lock().unwrap().push(spec.clone());
            Ok(())
        }
        async fn provision_user_in_tenant(
            &self,
            user: &User,
            tenant: &TenantId,
            role_name: &str,
            events: &[OutboxMessage],
        ) -> DomainResult<()> {
            self.record(events);
            self.users
                .lock()
                .unwrap()
                .push((user.clone(), tenant.clone(), role_name.to_string()));
            Ok(())
        }
        async fn update_user(&self, _user: &User, events: &[OutboxMessage]) -> DomainResult<()> {
            self.record(events);
            Ok(())
        }
    }

    struct StubClock;
    impl Clock for StubClock {
        fn now_utc(&self) -> DateTime<Utc> {
            Utc::now()
        }
    }

    // --- helpers ------------------------------------------------------------

    fn user(active: bool) -> User {
        User {
            id: UserId(Uuid::now_v7()),
            email: Email("a@b.com".into()),
            display_name: "A".into(),
            password_hash: "ph".into(),
            is_active: active,
            created_at: Utc::now(),
        }
    }

    fn role(name: &str, tenant: &str) -> Role {
        Role {
            id: RoleId(Uuid::now_v7()),
            tenant_id: TenantId(tenant.into()),
            name: name.into(),
            description: None,
        }
    }

    fn build(
        users: Vec<User>,
        tenants: Vec<&str>,
        roles: Vec<Role>,
        pw_ok: bool,
    ) -> (IdentityService, Arc<FakeRefreshRepo>) {
        let refresh = Arc::new(FakeRefreshRepo::default());
        let svc = IdentityService::new(
            Arc::new(FakeUserRepo { users }),
            Arc::new(FakeTenantRepo),
            Arc::new(FakeRoleRepo {
                tenants: tenants.into_iter().map(|t| TenantId(t.into())).collect(),
                roles,
            }),
            refresh.clone(),
            Arc::new(FakeProvisioningRepo::default()),
            Arc::new(FakeTokenIssuer),
            Arc::new(FakePasswordHasher { ok: pw_ok }),
            Arc::new(FakeRefreshHasher::default()),
            Arc::new(StubClock),
            1_209_600,
        );
        (svc, refresh)
    }

    // --- tests --------------------------------------------------------------

    #[tokio::test]
    async fn login_wrong_password_is_unauthorized() {
        let (svc, _) = build(
            vec![user(true)],
            vec!["t1"],
            vec![role("OWNER", "t1")],
            false,
        );
        let err = svc
            .login(Email("a@b.com".into()), "pw".into(), None)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Unauthorized(_)));
    }

    #[tokio::test]
    async fn login_inactive_user_is_unauthorized() {
        let (svc, _) = build(vec![user(false)], vec!["t1"], vec![], true);
        let err = svc
            .login(Email("a@b.com".into()), "pw".into(), None)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Unauthorized(_)));
    }

    #[tokio::test]
    async fn login_unknown_email_is_unauthorized() {
        let (svc, _) = build(vec![], vec![], vec![], true);
        let err = svc
            .login(Email("x@y.com".into()), "pw".into(), None)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Unauthorized(_)));
    }

    #[tokio::test]
    async fn login_multi_tenant_without_tenant_id_is_validation_error() {
        let (svc, _) = build(vec![user(true)], vec!["t1", "t2"], vec![], true);
        let err = svc
            .login(Email("a@b.com".into()), "pw".into(), None)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn login_happy_path_issues_access_and_refresh() {
        let (svc, refresh) = build(
            vec![user(true)],
            vec!["t1"],
            vec![role("OWNER", "t1"), role("ADMIN", "t1")],
            true,
        );
        let out = svc
            .login(Email("a@b.com".into()), "pw".into(), None)
            .await
            .unwrap();

        assert_eq!(out.tenant_id.0, "t1");
        assert_eq!(out.roles, vec!["OWNER".to_string(), "ADMIN".to_string()]);
        assert!(out.access.token.contains("OWNER"));
        assert_eq!(out.refresh_token, "raw-0");
        let stored = refresh.inserted.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].token_hash, "h:raw-0"); // only the hash is stored
    }

    #[tokio::test]
    async fn refresh_rotates_and_revokes_old_token() {
        let (svc, refresh) = build(
            vec![user(true)],
            vec!["t1"],
            vec![role("OWNER", "t1")],
            true,
        );
        let login = svc
            .login(Email("a@b.com".into()), "pw".into(), None)
            .await
            .unwrap();

        let out = svc.refresh(login.refresh_token.clone()).await.unwrap();

        assert_eq!(out.refresh_token, "raw-1");
        assert!(refresh
            .revoked
            .lock()
            .unwrap()
            .contains(&"h:raw-0".to_string()));
        assert_eq!(refresh.inserted.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn refresh_with_invalid_token_is_unauthorized() {
        let (svc, _) = build(vec![user(true)], vec!["t1"], vec![], true);
        let err = svc.refresh("nope".into()).await.unwrap_err();
        assert!(matches!(err, DomainError::Unauthorized(_)));
    }

    #[tokio::test]
    async fn logout_revokes_presented_token() {
        let (svc, refresh) = build(vec![user(true)], vec!["t1"], vec![], true);
        let login = svc
            .login(Email("a@b.com".into()), "pw".into(), None)
            .await
            .unwrap();

        svc.logout(login.refresh_token.clone()).await.unwrap();

        assert!(refresh
            .revoked
            .lock()
            .unwrap()
            .contains(&"h:raw-0".to_string()));
    }

    #[tokio::test]
    async fn register_provisions_tenant_owner_and_publishes_events() {
        let provisioning = Arc::new(FakeProvisioningRepo::default());
        let refresh = Arc::new(FakeRefreshRepo::default());
        let svc = IdentityService::new(
            Arc::new(FakeUserRepo { users: vec![] }),
            Arc::new(FakeTenantRepo),
            Arc::new(FakeRoleRepo {
                tenants: vec![],
                roles: vec![],
            }),
            refresh,
            provisioning.clone(),
            Arc::new(FakeTokenIssuer),
            Arc::new(FakePasswordHasher { ok: true }),
            Arc::new(FakeRefreshHasher::default()),
            Arc::new(StubClock),
            1_209_600,
        );

        let out = svc
            .register(
                "Acme".into(),
                None,
                Email("owner@acme.com".into()),
                "supersecret".into(),
                "Owner".into(),
            )
            .await
            .unwrap();

        // Sign-up returns a session for the owner with the OWNER role.
        assert_eq!(out.roles, vec!["OWNER".to_string()]);
        assert!(out.access.token.contains("OWNER"));

        // One provisioning spec: 5 roles, owner = OWNER, OWNER has full matrix.
        let specs = provisioning.specs.lock().unwrap();
        assert_eq!(specs.len(), 1);
        let spec = &specs[0];
        assert_eq!(spec.owner_role, "OWNER");
        assert_eq!(spec.roles.len(), 5);
        assert_eq!(spec.tenant.name, "Acme");
        assert_eq!(spec.tenant.plan, "free");
        assert_eq!(spec.owner.email.0, "owner@acme.com");
        let owner_role = spec.roles.iter().find(|r| r.name == "OWNER").unwrap();
        assert_eq!(
            owner_role.permission_codes.len(),
            rbac::ALL_PERMISSIONS.len()
        );

        // Both events enqueued in the outbox: TenantCreated then UserRegistered.
        assert_eq!(spec.events.len(), 2);
        assert_eq!(
            *provisioning.events.lock().unwrap(),
            vec!["TenantCreated".to_string(), "UserRegistered".to_string()]
        );
    }

    #[tokio::test]
    async fn register_rejects_short_password() {
        let (svc, _) = build(vec![], vec![], vec![], true);
        let err = svc
            .register(
                "Acme".into(),
                None,
                Email("o@a.com".into()),
                "short".into(),
                "Owner".into(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn register_rejects_invalid_email() {
        let (svc, _) = build(vec![], vec![], vec![], true);
        let err = svc
            .register(
                "Acme".into(),
                None,
                Email("not-an-email".into()),
                "supersecret".into(),
                "Owner".into(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn create_user_provisions_with_role_and_publishes_user_registered() {
        let provisioning = Arc::new(FakeProvisioningRepo::default());
        let svc = IdentityService::new(
            Arc::new(FakeUserRepo { users: vec![] }),
            Arc::new(FakeTenantRepo),
            Arc::new(FakeRoleRepo {
                tenants: vec![],
                roles: vec![],
            }),
            Arc::new(FakeRefreshRepo::default()),
            provisioning.clone(),
            Arc::new(FakeTokenIssuer),
            Arc::new(FakePasswordHasher { ok: true }),
            Arc::new(FakeRefreshHasher::default()),
            Arc::new(StubClock),
            1_209_600,
        );

        let view = svc
            .create_user(
                &TenantId("t1".into()),
                Email("staff@acme.com".into()),
                "supersecret".into(),
                "Staff".into(),
                "STAFF".into(),
            )
            .await
            .unwrap();

        assert_eq!(view.roles, vec!["STAFF".to_string()]);
        let provisioned = provisioning.users.lock().unwrap();
        assert_eq!(provisioned.len(), 1);
        assert_eq!(provisioned[0].1 .0, "t1");
        assert_eq!(provisioned[0].2, "STAFF");
        assert_eq!(
            *provisioning.events.lock().unwrap(),
            vec!["UserRegistered".to_string()]
        );
    }

    #[tokio::test]
    async fn create_user_rejects_unknown_role() {
        let (svc, _) = build(vec![], vec![], vec![], true);
        let err = svc
            .create_user(
                &TenantId("t1".into()),
                Email("x@y.com".into()),
                "supersecret".into(),
                "X".into(),
                "ROOT".into(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn update_user_changes_fields_and_publishes_user_updated() {
        let existing = user(true);
        let user_id = existing.id;
        let provisioning = Arc::new(FakeProvisioningRepo::default());
        let svc = IdentityService::new(
            Arc::new(FakeUserRepo {
                users: vec![existing],
            }),
            Arc::new(FakeTenantRepo),
            Arc::new(FakeRoleRepo {
                tenants: vec![],
                roles: vec![role("ADMIN", "t1")],
            }),
            Arc::new(FakeRefreshRepo::default()),
            provisioning.clone(),
            Arc::new(FakeTokenIssuer),
            Arc::new(FakePasswordHasher { ok: true }),
            Arc::new(FakeRefreshHasher::default()),
            Arc::new(StubClock),
            1_209_600,
        );

        let view = svc
            .update_user(
                &TenantId("t1".into()),
                &user_id,
                Some("New Name".into()),
                Some(false),
            )
            .await
            .unwrap();

        assert_eq!(view.user.display_name, "New Name");
        assert!(!view.user.is_active);
        assert_eq!(view.roles, vec!["ADMIN".to_string()]);
        assert_eq!(
            *provisioning.events.lock().unwrap(),
            vec!["UserUpdated".to_string()]
        );
    }

    #[tokio::test]
    async fn get_user_unknown_is_not_found() {
        let (svc, _) = build(vec![], vec![], vec![], true);
        let err = svc
            .get_user(&TenantId("t1".into()), &UserId(Uuid::now_v7()))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }

    fn service_with_tenant(tenant: Option<Tenant>) -> IdentityService {
        let tenant_repo = RecordingTenantRepo::default();
        *tenant_repo.tenant.lock().unwrap() = tenant;
        IdentityService::new(
            Arc::new(FakeUserRepo { users: vec![] }),
            Arc::new(tenant_repo),
            Arc::new(FakeRoleRepo {
                tenants: vec![],
                roles: vec![],
            }),
            Arc::new(FakeRefreshRepo::default()),
            Arc::new(FakeProvisioningRepo::default()),
            Arc::new(FakeTokenIssuer),
            Arc::new(FakePasswordHasher { ok: true }),
            Arc::new(FakeRefreshHasher::default()),
            Arc::new(StubClock),
            1_209_600,
        )
    }

    fn tenant(name: &str) -> Tenant {
        Tenant {
            id: TenantId("t1".into()),
            name: name.into(),
            plan: "free".into(),
            is_active: true,
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn get_tenant_returns_existing() {
        let svc = service_with_tenant(Some(tenant("Acme")));
        let got = svc.get_tenant(&TenantId("t1".into())).await.unwrap();
        assert_eq!(got.name, "Acme");
    }

    #[tokio::test]
    async fn get_tenant_missing_is_not_found() {
        let svc = service_with_tenant(None);
        let err = svc.get_tenant(&TenantId("t1".into())).await.unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn update_tenant_changes_fields() {
        let svc = service_with_tenant(Some(tenant("Acme")));
        let updated = svc
            .update_tenant(
                &TenantId("t1".into()),
                Some("NewName".into()),
                Some("pro".into()),
                Some(false),
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "NewName");
        assert_eq!(updated.plan, "pro");
        assert!(!updated.is_active);
    }
}
