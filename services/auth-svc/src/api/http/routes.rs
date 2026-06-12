//! Route table for auth-svc.
//!
//! All business routes live under the gateway-compatible prefix
//! `/api/v1/auth/...` (API-GATEWAY.md). `/metrics` is exposed at the root for
//! Prometheus scraping only (internal, never routed through the gateway).

use axum::middleware;
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
        // --- sign-up (public) + authentication (AUTH-FLOW.md, API-GATEWAY.md §3) ---
        .route("/register", post(handlers::auth::register))
        .route("/login", post(handlers::auth::login))
        .route("/refresh", post(handlers::auth::refresh))
        .route("/logout", post(handlers::auth::logout))
        .route("/me", get(handlers::auth::me))
        // --- admin user management (RBAC-guarded, API-GATEWAY.md §3.2) ---
        .route(
            "/users",
            get(handlers::users::list).post(handlers::users::create),
        )
        .route(
            "/users/{user_id}",
            get(handlers::users::get)
                .patch(handlers::users::update)
                .delete(handlers::users::deactivate),
        )
        // --- tenant management (own tenant, RBAC-guarded, API-GATEWAY.md §3.3) ---
        .route(
            "/tenants/{tenant_id}",
            get(handlers::tenants::get).patch(handlers::tenants::update),
        );

    Router::new()
        .nest("/api/v1/auth", auth_routes)
        .route("/metrics", get(handlers::metrics::render))
        // Standard HTTP metrics (OBSERVABILITY.md §4.3), shared across services
        // via platform-observability (was a local copy here).
        .layer(middleware::from_fn_with_state(
            service_name,
            platform_observability::track_http_metrics,
        ))
        .layer(TraceLayer::new_for_http().make_span_with(
            move |request: &axum::http::Request<_>| {
                // Root span `http.server` (OBSERVABILITY.md §5.4).
                // `tenant_id`/`user_id` are recorded later by the auth middleware
                // once the JWT is parsed.
                let span = tracing::info_span!(
                    "http.server",
                    service = service_name,
                    env = %env,
                    "http.method" = %request.method(),
                    "http.route" = %request.uri().path(),
                    tenant_id = tracing::field::Empty,
                    user_id = tracing::field::Empty,
                );
                // Continue the upstream trace if a `traceparent` was forwarded
                // (OBSERVABILITY.md §5.3).
                let header = |name| request.headers().get(name).and_then(|v| v.to_str().ok());
                platform_observability::set_parent_from_w3c(
                    &span,
                    header("traceparent"),
                    header("tracestate"),
                );
                span
            },
        ))
        .with_state(state)
}
