//! Route table for crm-svc.
//!
//! Business routes live under the gateway prefix `/api/v1/crm/...`
//! (ARCHITECTURE.md §3.3). `/metrics` is exposed at the root for Prometheus.

use axum::routing::{get, post};
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::api::http::handlers;
use crate::bootstrap::wiring::AppState;

pub fn router(state: AppState) -> Router {
    let service_name = state.config.service_name;
    let env = state.config.env.clone();

    let crm_routes = Router::new()
        .route("/health", get(handlers::health::health))
        .route("/ready", get(handlers::health::ready))
        // --- customers (RBAC-guarded, ARCHITECTURE.md §3.3) ---
        .route(
            "/customers",
            get(handlers::customers::list).post(handlers::customers::create),
        )
        .route(
            "/customers/{customer_id}",
            get(handlers::customers::get).patch(handlers::customers::update),
        )
        // --- deals / sales pipeline (RBAC-guarded, ARCHITECTURE.md §3.3) ---
        .route(
            "/deals",
            get(handlers::deals::list).post(handlers::deals::create),
        )
        .route("/deals/{deal_id}", get(handlers::deals::get))
        .route(
            "/deals/{deal_id}/stage",
            post(handlers::deals::change_stage),
        )
        // --- contacts (RBAC-guarded, ARCHITECTURE.md §3.3) ---
        .route(
            "/contacts",
            get(handlers::contacts::list).post(handlers::contacts::create),
        )
        .route(
            "/contacts/{contact_id}",
            get(handlers::contacts::get).patch(handlers::contacts::update),
        );
    // Further slices (activities) add their routes here.

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
