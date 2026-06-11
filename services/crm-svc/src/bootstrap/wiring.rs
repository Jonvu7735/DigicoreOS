//! Dependency wiring (DI) for crm-svc. The ONLY place infra is bound to
//! domain ports. Handlers receive everything via [`AppState`].

use std::sync::Arc;

use axum::Router;
use platform_auth::JwtVerifier;
use platform_observability::PrometheusHandle;
use platform_outbox::{connect_publisher, OutboxRelay, OutboxRepository, PgOutboxRepo};
use sqlx::PgPool;

use crate::api;
use crate::bootstrap::config::AppConfig;
use crate::domain::contacts::services::ContactService;
use crate::domain::customers::ports::CustomerRepository;
use crate::domain::customers::services::CustomerService;
use crate::domain::deals::services::DealService;
use crate::domain::shared::types::Clock;
use crate::infra;
use crate::infra::db::contact_repo_pg::PgContactRepo;
use crate::infra::db::customer_repo_pg::PgCustomerRepo;
use crate::infra::db::deal_repo_pg::PgDealRepo;
use crate::infra::time::clock::SystemClock;

/// Shared application state injected into every handler.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: PgPool,
    pub metrics: PrometheusHandle,
    /// Verifies the RS256 access tokens issued by auth-svc.
    pub verifier: Arc<JwtVerifier>,
    pub customers: Arc<CustomerService>,
    pub deals: Arc<DealService>,
    pub contacts: Arc<ContactService>,
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
    let customer_repo: Arc<dyn CustomerRepository> = Arc::new(PgCustomerRepo::new(db.clone()));
    let customers = Arc::new(CustomerService::new(customer_repo.clone(), clock.clone()));
    let deals = Arc::new(DealService::new(
        Arc::new(PgDealRepo::new(db.clone())),
        customer_repo.clone(),
        clock.clone(),
    ));
    let contacts = Arc::new(ContactService::new(
        Arc::new(PgContactRepo::new(db.clone())),
        customer_repo,
        clock,
    ));

    // Outbox relay (DATA-STRATEGY.md §3.2): only runs when NATS is reachable;
    // otherwise events accumulate in `outbox_events` until a relay drains them.
    let outbox_repo: Arc<dyn OutboxRepository> = Arc::new(PgOutboxRepo::new(db.clone()));
    if let Some(publisher) = connect_publisher(config.nats_url.as_deref()).await {
        tokio::spawn(OutboxRelay::new(outbox_repo, publisher).run());
        tracing::info!("outbox relay started");
    }

    Ok(AppState {
        config,
        db,
        metrics,
        verifier,
        customers,
        deals,
        contacts,
    })
}

/// Build the Axum router (delegates to `api/http/routes.rs`).
pub fn build_router(state: AppState) -> Router {
    api::http::routes::router(state)
}
