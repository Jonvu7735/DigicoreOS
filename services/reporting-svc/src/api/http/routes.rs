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
    let env = state.config.env.clone();

    let reporting_routes = Router::new()
        .route("/health", get(handlers::health::health))
        .route("/ready", get(handlers::health::ready))
        // --- dashboards (RBAC-guarded, ARCHITECTURE.md §3.5) ---
        .route("/sales-summary", get(handlers::sales::summary));
    // Further read-model slices (overview, crm-funnel, hrm-summary, …) add
    // their RBAC-guarded routes here as they land.

    Router::new()
        .nest("/api/v1/reporting", reporting_routes)
        .route("/metrics", get(handlers::metrics::render))
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
