//! Dependency wiring (DI) for hrm-svc. The ONLY place infra is bound to
//! domain ports. Handlers receive everything via [`AppState`].

use std::sync::Arc;

use axum::Router;
use platform_auth::JwtVerifier;
use platform_observability::PrometheusHandle;
use platform_outbox::{connect_publisher, OutboxRelay, OutboxRepository, PgOutboxRepo};
use sqlx::PgPool;

use crate::api;
use crate::bootstrap::config::AppConfig;
use crate::domain::attendance::services::AttendanceService;
use crate::domain::employees::ports::EmployeeRepository;
use crate::domain::employees::services::EmployeeService;
use crate::domain::leave::services::LeaveService;
use crate::domain::shared::types::Clock;
use crate::infra;
use crate::infra::db::attendance_repo_pg::PgAttendanceRepo;
use crate::infra::db::employee_repo_pg::PgEmployeeRepo;
use crate::infra::db::leave_repo_pg::PgLeaveRepo;
use crate::infra::time::clock::SystemClock;

/// Shared application state injected into every handler.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: PgPool,
    pub metrics: PrometheusHandle,
    /// Verifies the RS256 access tokens issued by auth-svc.
    pub verifier: Arc<JwtVerifier>,
    pub employees: Arc<EmployeeService>,
    pub attendance: Arc<AttendanceService>,
    pub leave: Arc<LeaveService>,
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
    let employee_repo: Arc<dyn EmployeeRepository> = Arc::new(PgEmployeeRepo::new(db.clone()));
    let employees = Arc::new(EmployeeService::new(employee_repo.clone(), clock.clone()));
    let attendance = Arc::new(AttendanceService::new(
        Arc::new(PgAttendanceRepo::new(db.clone())),
        employee_repo.clone(),
        clock.clone(),
    ));
    let leave = Arc::new(LeaveService::new(
        Arc::new(PgLeaveRepo::new(db.clone())),
        employee_repo,
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
        employees,
        attendance,
        leave,
    })
}

/// Build the Axum router (delegates to `api/http/routes.rs`).
pub fn build_router(state: AppState) -> Router {
    api::http::routes::router(state)
}
