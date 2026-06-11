//! Route table for core-erp-svc.
//!
//! Business routes live under the gateway prefix `/api/v1/erp/...`
//! (API-GATEWAY.md §4). `/metrics` is exposed at the root for Prometheus.

use axum::routing::get;
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::api::http::handlers;
use crate::bootstrap::wiring::AppState;

pub fn router(state: AppState) -> Router {
    let service_name = state.config.service_name;
    let env = state.config.env.clone();

    let erp_routes = Router::new()
        .route("/health", get(handlers::health::health))
        .route("/ready", get(handlers::health::ready))
        // --- products (RBAC-guarded, API-GATEWAY.md §4.4) ---
        .route(
            "/products",
            get(handlers::products::list).post(handlers::products::create),
        )
        .route(
            "/products/{product_id}",
            get(handlers::products::get).patch(handlers::products::update),
        );
    // TODO(Phase 3 cont.): orders, payments, inventory, invoices (API-GATEWAY.md §4).

    Router::new()
        .nest("/api/v1/erp", erp_routes)
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
