//! Route table for crm-svc.
//!
//! Business routes live under the gateway prefix `/api/v1/crm/...`
//! (ARCHITECTURE.md §3.3). `/metrics` is exposed at the root for Prometheus.

use axum::routing::get;
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::api::http::handlers;
use crate::bootstrap::wiring::AppState;

pub fn router(state: AppState) -> Router {
    let service_name = state.config.service_name;
    let env = state.config.env.clone();

    let crm_routes = Router::new()
        .route("/health", get(handlers::health::health))
        .route("/ready", get(handlers::health::ready));
    // Domain slices (customers, contacts, deals, activities) add their
    // RBAC-guarded routes here as they land.

    Router::new()
        .nest("/api/v1/crm", crm_routes)
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
