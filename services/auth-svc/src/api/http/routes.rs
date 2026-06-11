//! Route table for auth-svc.
//!
//! All business routes live under the gateway-compatible prefix
//! `/api/v1/auth/...` (API-GATEWAY.md). `/metrics` is exposed at the root for
//! Prometheus scraping only (internal, never routed through the gateway).

use std::time::Instant;

use axum::extract::{MatchedPath, Request};
use axum::middleware::{self, Next};
use axum::response::Response;
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
        .layer(middleware::from_fn(track_http_metrics))
        .layer(TraceLayer::new_for_http().make_span_with(
            move |request: &axum::http::Request<_>| {
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
            },
        ))
        .with_state(state)
}

/// Records `http_requests_total` and `http_request_duration_seconds` for every
/// request (OBSERVABILITY.md §4.3). Phase 1.2 routes are static; the `path`
/// label uses `MatchedPath` when present, else the raw path (switch fully to
/// `MatchedPath` in Phase 1.3 when routes gain `{id}` params).
async fn track_http_metrics(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|m| m.as_str().to_owned())
        .unwrap_or_else(|| req.uri().path().to_owned());

    let start = Instant::now();
    let response = next.run(req).await;
    let status = response.status().as_u16().to_string();

    metrics::histogram!(
        "http_request_duration_seconds",
        "service" => "auth-svc",
        "method" => method.to_string(),
        "path" => path.clone(),
    )
    .record(start.elapsed().as_secs_f64());
    metrics::counter!(
        "http_requests_total",
        "service" => "auth-svc",
        "method" => method.to_string(),
        "path" => path,
        "status" => status,
    )
    .increment(1);

    response
}
