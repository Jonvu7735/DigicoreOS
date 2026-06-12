//! Route table for reporting-svc.
//!
//! Business routes live under the gateway prefix `/api/v1/reporting/...`
//! (ARCHITECTURE.md §3.5). `/metrics` is exposed at the root for Prometheus.

use axum::routing::get;
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::api::http::handlers;
use crate::bootstrap::wiring::AppState;

pub fn router(state: AppState) -> Router {
    let service_name = state.config.service_name;
    // Per-tenant rate limiter (SECURITY.md §5.2), reuses the JWT verifier.
    let limiter =
        platform_ratelimit::TenantRateLimiter::from_env(state.verifier.clone(), service_name);
    let env = state.config.env.clone();

    let reporting_routes = Router::new()
        .route("/health", get(handlers::health::health))
        .route("/ready", get(handlers::health::ready))
        // --- dashboards (RBAC-guarded, ARCHITECTURE.md §3.5) ---
        .route("/overview", get(handlers::overview::overview))
        .route("/sales-summary", get(handlers::sales::summary))
        .route("/orders", get(handlers::orders::list))
        .route("/customers", get(handlers::customers::list))
        .route("/employees", get(handlers::employees::list))
        .route("/crm-funnel", get(handlers::crm_funnel::funnel))
        .route(
            "/inventory-summary",
            get(handlers::inventory_summary::summary),
        )
        .route("/hrm-summary", get(handlers::hrm_summary::summary))
        // --- snapshots (capture a read model + emit ReportSnapshotCreated) ---
        .route(
            "/snapshots",
            get(handlers::snapshots::list).post(handlers::snapshots::create),
        );
    // Further read-model slices (crm-funnel, hrm-summary, …) add their
    // RBAC-guarded routes here as the matching projections land.

    Router::new()
        .nest("/api/v1/reporting", reporting_routes)
        .route("/metrics", get(handlers::metrics::render))
        // Per-tenant rate limiting (SECURITY.md §5.2).
        .layer(axum::middleware::from_fn_with_state(
            limiter,
            platform_ratelimit::tenant_rate_limit,
        ))
        // Standard HTTP metrics (OBSERVABILITY.md §4.3), shared across services.
        .layer(axum::middleware::from_fn_with_state(
            service_name,
            platform_observability::track_http_metrics,
        ))
        .layer(TraceLayer::new_for_http().make_span_with(
            move |request: &axum::http::Request<_>| {
                tracing::info_span!(
                    "http.server",
                    service = service_name,
                    env = %env,
                    "http.method" = %request.method(),
                    "http.route" = %request.uri().path(),
                    tenant_id = tracing::field::Empty,
                    user_id = tracing::field::Empty,
                )
            },
        ))
        .with_state(state)
}
