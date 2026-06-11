//! Dependency wiring (DI) for core-erp-svc. The ONLY place infra is bound to
//! domain ports. Handlers receive everything via [`AppState`].

use std::sync::Arc;

use axum::Router;
use platform_auth::JwtVerifier;
use platform_observability::PrometheusHandle;
use sqlx::PgPool;

use crate::api;
use crate::bootstrap::config::AppConfig;
use crate::domain::products::services::ProductService;
use crate::domain::shared::types::Clock;
use crate::infra;
use crate::infra::db::product_repo_pg::PgProductRepo;
use crate::infra::time::clock::SystemClock;

/// Shared application state injected into every handler.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: PgPool,
    pub metrics: PrometheusHandle,
    /// Verifies the RS256 access tokens issued by auth-svc.
    pub verifier: Arc<JwtVerifier>,
    pub products: Arc<ProductService>,
}

/// Construct infrastructure adapters and bind them to domain ports.
pub async fn build_app_state(config: AppConfig) -> anyhow::Result<AppState> {
    let config = Arc::new(config);

    let metrics = platform_observability::install_prometheus()?;

    let db = infra::db::postgres::connect_lazy(&config)?;
    infra::db::postgres::run_migrations(&db).await?;

    let verifier = Arc::new(JwtVerifier::from_public_key_pem(
        &config.jwt.public_key_pem,
        &config.jwt.issuer,
        &config.jwt.audience,
    )?);

    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let products = Arc::new(ProductService::new(
        Arc::new(PgProductRepo::new(db.clone())),
        clock,
    ));

    Ok(AppState {
        config,
        db,
        metrics,
        verifier,
        products,
    })
}

/// Build the Axum router (delegates to `api/http/routes.rs`).
pub fn build_router(state: AppState) -> Router {
    api::http::routes::router(state)
}
