//! Route table for auth-svc.
//!
//! All business routes live under the gateway-compatible prefix
//! `/api/v1/auth/...` (API-GATEWAY.md). `/metrics` is exposed at the root for
//! Prometheus scraping only (internal, never routed through the gateway).

use axum::routing::{get, post};
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::api::http::handlers;
use crate::bootstrap::wiring::AppState;

pub fn router(state: AppState) -> Router {
    // Captured into the per-request root span so every JSON log line carries
    // `service` and `env` (OBSERVABILITY.md §3.2).
    let service_name = state.config.service_name;
    let env = state.config.env.clone();

    let auth_routes = Router::new()
        // --- liveness / readiness ---
        .route("/health", get(handlers::health::health))
        .route("/ready", get(handlers::health::ready))
        // --- authentication (AUTH-FLOW.md) ---
        .route("/login", post(handlers::auth::login))
        .route("/refresh", post(handlers::auth::refresh))
        .route("/logout", post(handlers::auth::logout))
        .route("/me", get(handlers::auth::me));
    // TODO(Phase 1.3): user management  GET/POST/PATCH/DELETE /users/...
    // TODO(Phase 1.3): tenant management GET/POST/PATCH       /tenants/...
    // (route list per API-GATEWAY.md group `/api/v1/auth`)

    Router::new()
        .nest("/api/v1/auth", auth_routes)
        .route("/metrics", get(handlers::metrics::render))
        .layer(
            TraceLayer::new_for_http().make_span_with(move |request: &axum::http::Request<_>| {
                // Root span `http.server` (OBSERVABILITY.md §5.4).
                // `tenant_id`/`user_id` are recorded later by the auth
                // middleware once the JWT is parsed (Phase 1.3).
                tracing::info_span!(
                    "http.server",
                    service = service_name,
                    env = %env,
                    "http.method" = %request.method(),
                    "http.route" = %request.uri().path(),
                    tenant_id = tracing::field::Empty,
                    user_id = tracing::field::Empty,
                )
            }),
        )
        .with_state(state)
}
