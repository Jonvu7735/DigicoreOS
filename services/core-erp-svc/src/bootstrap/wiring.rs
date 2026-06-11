//! Dependency wiring (DI) for core-erp-svc. The ONLY place infra is bound to
//! domain ports. Handlers receive everything via [`AppState`].

use std::sync::Arc;

use axum::Router;
use platform_auth::JwtVerifier;
use platform_observability::PrometheusHandle;
use platform_outbox::{OutboxRelay, OutboxRepository, PgOutboxRepo};
use sqlx::PgPool;

use crate::api;
use crate::bootstrap::config::AppConfig;
use crate::domain::orders::ports::OrderRepository;
use crate::domain::orders::services::OrderService;
use crate::domain::payments::services::PaymentService;
use crate::domain::products::services::ProductService;
use crate::domain::shared::types::Clock;
use crate::infra;
use crate::infra::db::order_repo_pg::PgOrderRepo;
use crate::infra::db::payment_repo_pg::PgPaymentRepo;
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
    pub orders: Arc<OrderService>,
    pub payments: Arc<PaymentService>,
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
        clock.clone(),
    ));
    let order_repo: Arc<dyn OrderRepository> = Arc::new(PgOrderRepo::new(db.clone()));
    let orders = Arc::new(OrderService::new(order_repo.clone(), clock.clone()));
    let payments = Arc::new(PaymentService::new(
        Arc::new(PgPaymentRepo::new(db.clone())),
        order_repo,
        clock,
    ));

    // Outbox relay (DATA-STRATEGY.md §3.2): only runs when NATS is reachable;
    // otherwise events accumulate in `outbox_events` until a relay drains them.
    let outbox_repo: Arc<dyn OutboxRepository> = Arc::new(PgOutboxRepo::new(db.clone()));
    if let Some(publisher) = platform_outbox::connect_publisher(config.nats_url.as_deref()).await {
        tokio::spawn(OutboxRelay::new(outbox_repo, publisher).run());
        tracing::info!("outbox relay started");
    }

    Ok(AppState {
        config,
        db,
        metrics,
        verifier,
        products,
        orders,
        payments,
    })
}

/// Build the Axum router (delegates to `api/http/routes.rs`).
pub fn build_router(state: AppState) -> Router {
    api::http::routes::router(state)
}
