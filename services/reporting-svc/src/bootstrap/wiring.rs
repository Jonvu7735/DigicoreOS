//! Dependency wiring (DI) for reporting-svc. The ONLY place infra is bound to
//! domain ports. Handlers receive everything via [`AppState`].

use std::sync::Arc;

use axum::Router;
use platform_auth::JwtVerifier;
use platform_observability::PrometheusHandle;
use platform_outbox::{connect_publisher, OutboxRelay, OutboxRepository, PgOutboxRepo};
use sqlx::PgPool;

use platform_events::{connect_consumer, InboundEventHandler, NatsConsumer};

use crate::api;
use crate::bootstrap::config::AppConfig;
use crate::domain::customers::ports::CustomersProjection;
use crate::domain::ingest::ingestor::EventIngestor;
use crate::domain::orders::ports::OrdersProjection;
use crate::domain::sales::ports::SalesProjection;
use crate::domain::shared::types::Clock;
use crate::domain::snapshots::services::SnapshotService;
use crate::infra;
use crate::infra::db::customers_repo_pg::PgCustomersRepo;
use crate::infra::db::orders_repo_pg::PgOrdersRepo;
use crate::infra::db::sales_repo_pg::PgSalesRepo;
use crate::infra::db::snapshot_repo_pg::PgSnapshotRepo;
use crate::infra::time::clock::SystemClock;

/// Shared application state injected into every handler.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: PgPool,
    pub metrics: PrometheusHandle,
    /// Verifies the RS256 access tokens issued by auth-svc.
    pub verifier: Arc<JwtVerifier>,
    /// Sales read model (queried by the dashboard, written by the consumer).
    pub sales: Arc<dyn SalesProjection>,
    /// Orders read model (per-order facts projected from OrderCreated).
    pub orders: Arc<dyn OrdersProjection>,
    /// Customers read model (per-customer facts projected from CustomerCreated).
    pub customers: Arc<dyn CustomersProjection>,
    pub snapshots: Arc<SnapshotService>,
}

/// Construct infrastructure adapters and bind them to domain ports.
pub async fn build_app_state(config: AppConfig) -> anyhow::Result<AppState> {
    let config = Arc::new(config);

    let metrics = platform_observability::install_prometheus()?;

    let db = infra::db::postgres::connect_lazy(&config)?;
    infra::db::postgres::run_migrations(&db, &config.database_schema).await?;

    let verifier = Arc::new(JwtVerifier::from_public_key_pem(
        &config.jwt.public_key_pem,
        &config.jwt.issuer,
        &config.jwt.audience,
    )?);

    // Read models + the ingestor that updates them from inbound events.
    let sales: Arc<dyn SalesProjection> = Arc::new(PgSalesRepo::new(db.clone()));
    let orders: Arc<dyn OrdersProjection> = Arc::new(PgOrdersRepo::new(db.clone()));
    let customers: Arc<dyn CustomersProjection> = Arc::new(PgCustomersRepo::new(db.clone()));
    let ingestor: Arc<dyn InboundEventHandler> = Arc::new(EventIngestor::new(
        sales.clone(),
        orders.clone(),
        customers.clone(),
    ));

    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let snapshots = Arc::new(SnapshotService::new(
        Arc::new(PgSnapshotRepo::new(db.clone())),
        sales.clone(),
        clock,
    ));

    // Outbox relay (DATA-STRATEGY.md §3.2): reporting publishes ReportSnapshotCreated.
    let outbox_repo: Arc<dyn OutboxRepository> = Arc::new(PgOutboxRepo::new(db.clone()));
    if let Some(publisher) = connect_publisher(config.nats_url.as_deref()).await {
        tokio::spawn(OutboxRelay::new(outbox_repo, publisher).run());
        tracing::info!("outbox relay started");
    }

    // Inbound consumer: drain platform.> into the read models (when NATS is up).
    if let Some(client) = connect_consumer(config.nats_url.as_deref()).await {
        tokio::spawn(NatsConsumer::new(client, ingestor).run());
        tracing::info!("event consumer started");
    }

    Ok(AppState {
        config,
        db,
        metrics,
        verifier,
        sales,
        orders,
        customers,
        snapshots,
    })
}

/// Build the Axum router (delegates to `api/http/routes.rs`).
pub fn build_router(state: AppState) -> Router {
    api::http::routes::router(state)
}
