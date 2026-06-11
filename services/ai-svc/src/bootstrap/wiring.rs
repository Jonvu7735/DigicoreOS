//! Dependency wiring (DI) for ai-svc. The ONLY place infra is bound to domain
//! ports. Handlers receive everything via [`AppState`].

use std::sync::Arc;

use axum::Router;
use platform_auth::JwtVerifier;
use platform_events::{connect_consumer, InboundEventHandler, NatsConsumer};
use platform_observability::PrometheusHandle;
use platform_outbox::{connect_publisher, OutboxRelay, OutboxRepository, PgOutboxRepo};
use sqlx::PgPool;

use crate::api;
use crate::bootstrap::config::AppConfig;
use crate::domain::ingest::ingestor::EventIngestor;
use crate::domain::insights::ports::{InsightGenerator, InsightRepository};
use crate::domain::insights::services::InsightService;
use crate::domain::shared::types::Clock;
use crate::infra;
use crate::infra::ai::stub_generator::StubInsightGenerator;
use crate::infra::db::insight_repo_pg::PgInsightRepo;
use crate::infra::time::clock::SystemClock;

/// Shared application state injected into every handler.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: PgPool,
    pub metrics: PrometheusHandle,
    /// Verifies the RS256 access tokens issued by auth-svc.
    pub verifier: Arc<JwtVerifier>,
    pub insights: Arc<InsightService>,
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

    // Insight engine: a stub generator behind the InsightGenerator port (a real
    // LLM adapter slots in here without touching domain/api).
    let generator: Arc<dyn InsightGenerator> = Arc::new(StubInsightGenerator);
    let insight_repo: Arc<dyn InsightRepository> = Arc::new(PgInsightRepo::new(db.clone()));
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let insights = Arc::new(InsightService::new(insight_repo, generator, clock));

    // Outbox relay (DATA-STRATEGY.md §3.2): ai-svc publishes AiInsightGenerated.
    let outbox_repo: Arc<dyn OutboxRepository> = Arc::new(PgOutboxRepo::new(db.clone()));
    if let Some(publisher) = connect_publisher(config.nats_url.as_deref()).await {
        tokio::spawn(OutboxRelay::new(outbox_repo, publisher).run());
        tracing::info!("outbox relay started");
    }

    // Inbound consumer: react to platform events (e.g. ReportSnapshotCreated) by
    // generating insights (when NATS is reachable).
    let ingestor: Arc<dyn InboundEventHandler> = Arc::new(EventIngestor::new(insights.clone()));
    if let Some(client) = connect_consumer(config.nats_url.as_deref()).await {
        tokio::spawn(NatsConsumer::new(client, ingestor).run());
        tracing::info!("event consumer started");
    }

    Ok(AppState {
        config,
        db,
        metrics,
        verifier,
        insights,
    })
}

/// Build the Axum router (delegates to `api/http/routes.rs`).
pub fn build_router(state: AppState) -> Router {
    api::http::routes::router(state)
}
