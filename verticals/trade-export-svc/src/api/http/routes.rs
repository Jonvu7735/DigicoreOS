//! Route table for trade-export-svc.
//!
//! Business routes live under the gateway prefix `/api/v1/trade-export/...`.
//! `/metrics` is exposed at the root for Prometheus.

use axum::routing::{get, post};
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

    let routes = Router::new()
        .route("/health", get(handlers::health::health))
        .route("/ready", get(handlers::health::ready))
        // --- export shipments ---
        .route(
            "/shipments",
            get(handlers::shipments::list).post(handlers::shipments::create),
        )
        .route("/shipments/{shipment_id}", get(handlers::shipments::get))
        .route(
            "/shipments/{shipment_id}/cargo",
            get(handlers::cargo::list).post(handlers::cargo::add),
        )
        .route(
            "/shipments/{shipment_id}/history",
            get(handlers::shipments::history),
        )
        .route(
            "/shipments/{shipment_id}/book",
            post(handlers::shipments::book),
        )
        .route(
            "/shipments/{shipment_id}/dispatch",
            post(handlers::shipments::dispatch),
        )
        .route(
            "/shipments/{shipment_id}/cancel",
            post(handlers::shipments::cancel),
        );

    Router::new()
        .nest("/api/v1/trade-export", routes)
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
