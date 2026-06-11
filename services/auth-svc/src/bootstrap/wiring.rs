//! Dependency wiring (DI) for auth-svc.
//!
//! This is the ONLY place where concrete `infra` implementations are bound to
//! `domain` ports. Handlers receive everything through [`AppState`]; they never
//! construct repositories, clients, or clocks themselves.

use std::sync::Arc;

use axum::Router;
use platform_observability::PrometheusHandle;
use sqlx::PgPool;

use crate::api;
use crate::bootstrap::config::AppConfig;
use crate::domain::identity::ports::{
    EventPublisher, PasswordHasher, ProvisioningRepository, RefreshTokenHasher,
    RefreshTokenRepository, RoleRepository, TenantRepository, TokenIssuer, UserRepository,
};
use crate::domain::identity::services::IdentityService;
use crate::domain::shared::types::Clock;
use crate::infra;
use crate::infra::db::provisioning_repo_pg::PgProvisioningRepo;
use crate::infra::db::refresh_token_repo_pg::PgRefreshTokenRepo;
use crate::infra::db::role_repo_pg::PgRoleRepo;
use crate::infra::db::tenant_repo_pg::PgTenantRepo;
use crate::infra::db::user_repo_pg::PgUserRepo;
use crate::infra::security::jwt::JwtTokenIssuer;
use crate::infra::security::password::Argon2PasswordHasher;
use crate::infra::security::refresh_token::Sha256RefreshTokenHasher;
use crate::infra::time::clock::SystemClock;

/// Shared application state injected into every handler.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: PgPool,
    pub metrics: PrometheusHandle,
    /// Identity bounded-context service (login/refresh/logout/users/tenants/RBAC).
    pub identity: Arc<IdentityService>,
}

/// Construct all infrastructure adapters and bind them to domain ports.
pub async fn build_app_state(config: AppConfig) -> anyhow::Result<AppState> {
    let config = Arc::new(config);

    // Observability: global Prometheus recorder (rendered at GET /metrics).
    let metrics = platform_observability::install_prometheus()?;

    // Postgres pool, pinned to this service's schema (DATA-STRATEGY.md §3.1).
    let db = infra::db::postgres::connect_lazy(&config)?;
    // Apply schema migrations at startup (fatal if the DB is unreachable: the
    // service cannot serve auth without its schema).
    infra::db::postgres::run_migrations(&db).await?;

    // --- infra adapters bound to domain ports ---
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let event_publisher: Arc<dyn EventPublisher> =
        infra::messaging::nats::build_publisher(config.nats_url.as_deref()).await;
    let user_repo: Arc<dyn UserRepository> = Arc::new(PgUserRepo::new(db.clone()));
    let tenant_repo: Arc<dyn TenantRepository> = Arc::new(PgTenantRepo::new(db.clone()));
    let role_repo: Arc<dyn RoleRepository> = Arc::new(PgRoleRepo::new(db.clone()));
    let refresh_token_repo: Arc<dyn RefreshTokenRepository> =
        Arc::new(PgRefreshTokenRepo::new(db.clone()));
    let provisioning: Arc<dyn ProvisioningRepository> =
        Arc::new(PgProvisioningRepo::new(db.clone()));
    let token_issuer: Arc<dyn TokenIssuer> = Arc::new(JwtTokenIssuer::from_config(&config.jwt)?);
    let password_hasher: Arc<dyn PasswordHasher> = Arc::new(Argon2PasswordHasher);
    let refresh_token_hasher: Arc<dyn RefreshTokenHasher> = Arc::new(Sha256RefreshTokenHasher);

    // --- domain services ---
    let identity = Arc::new(IdentityService::new(
        user_repo,
        tenant_repo,
        role_repo,
        refresh_token_repo,
        provisioning,
        token_issuer,
        password_hasher,
        refresh_token_hasher,
        event_publisher,
        clock,
        config.jwt.refresh_ttl_secs,
    ));

    Ok(AppState {
        config,
        db,
        metrics,
        identity,
    })
}

/// Build the Axum router (delegates to `api/http/routes.rs`).
pub fn build_router(state: AppState) -> Router {
    api::http::routes::router(state)
}
