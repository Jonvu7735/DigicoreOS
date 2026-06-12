//! Dependency wiring (DI) for trade-export-svc. The ONLY place infra is bound to
//! domain ports. Handlers receive everything via [`AppState`].

use std::sync::Arc;

use axum::Router;
use platform_auth::JwtVerifier;
use platform_events::{connect_consumer, InboundEventHandler, NatsConsumer};
use platform_observability::PrometheusHandle;
use platform_outbox::{connect_publisher, OutboxRelay, OutboxRepository, PgOutboxRepo};
use sqlx::PgPool;

use crate::api;
use crate::bootstrap::config::AppConfig;
use crate::domain::ingest::ingestor::ShipmentIngestor;
use crate::domain::shared::types::Clock;
use crate::domain::shipments::ports::{CargoLineRepository, ShipmentRepository};
use crate::domain::shipments::services::ShipmentService;
use crate::infra;
use crate::infra::db::cargo_line_repo_pg::PgCargoLineRepo;
use crate::infra::db::shipment_repo_pg::PgShipmentRepo;
use crate::infra::time::clock::SystemClock;

/// Shared application state injected into every handler.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: PgPool,
    pub metrics: PrometheusHandle,
    /// Verifies the RS256 access tokens issued by auth-svc.
    pub verifier: Arc<JwtVerifier>,
    pub shipments: Arc<ShipmentService>,
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

    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let repo: Arc<dyn ShipmentRepository> = Arc::new(PgShipmentRepo::new(db.clone()));
    let cargo: Arc<dyn CargoLineRepository> = Arc::new(PgCargoLineRepo::new(db.clone()));
    let shipments = Arc::new(ShipmentService::new(repo, cargo, clock));

    // Outbox relay (DATA-STRATEGY.md §3.2): publishes ShipmentBooked when NATS is up.
    let outbox_repo: Arc<dyn OutboxRepository> = Arc::new(PgOutboxRepo::new(db.clone()));
    if let Some(publisher) = connect_publisher(config.nats_url.as_deref()).await {
        tokio::spawn(OutboxRelay::new(outbox_repo, publisher).run());
        tracing::info!("outbox relay started");
    }

    // Inbound consumer: core ERP order.paid -> draft an export shipment (when NATS up).
    if let Some(client) = connect_consumer(config.nats_url.as_deref()).await {
        let ingestor: Arc<dyn InboundEventHandler> =
            Arc::new(ShipmentIngestor::new(shipments.clone()));
        tokio::spawn(NatsConsumer::new(client, ingestor).run());
        tracing::info!("event consumer started");
    }

    Ok(AppState {
        config,
        db,
        metrics,
        verifier,
        shipments,
    })
}

/// Build the Axum router (delegates to `api/http/routes.rs`).
pub fn build_router(state: AppState) -> Router {
    api::http::routes::router(state)
}
