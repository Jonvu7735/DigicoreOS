//! Route table for ai-svc.
//!
//! Business routes live under the gateway prefix `/api/v1/ai/...`
//! (ARCHITECTURE.md §3.6). `/metrics` is exposed at the root for Prometheus.

use axum::routing::{get, post};
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::api::http::handlers;
use crate::bootstrap::wiring::AppState;

pub fn router(state: AppState) -> Router {
    let service_name = state.config.service_name;
    let env = state.config.env.clone();

    let ai_routes = Router::new()
        .route("/health", get(handlers::health::health))
        .route("/ready", get(handlers::health::ready))
        // --- insights (RBAC-guarded, ARCHITECTURE.md §3.6) ---
        .route("/insight", post(handlers::insights::generate))
        .route("/insights", get(handlers::insights::list));
    // Further engine slices (query, assist, models) add their RBAC-guarded
    // routes here as they land.

    Router::new()
        .nest("/api/v1/ai", ai_routes)
        .route("/metrics", get(handlers::metrics::render))
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
