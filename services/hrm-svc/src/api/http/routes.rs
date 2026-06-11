//! Route table for hrm-svc.
//!
//! Business routes live under the gateway prefix `/api/v1/hrm/...`
//! (ARCHITECTURE.md §3.4). `/metrics` is exposed at the root for Prometheus.

use axum::routing::{get, post};
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::api::http::handlers;
use crate::bootstrap::wiring::AppState;

pub fn router(state: AppState) -> Router {
    let service_name = state.config.service_name;
    let env = state.config.env.clone();

    let hrm_routes = Router::new()
        .route("/health", get(handlers::health::health))
        .route("/ready", get(handlers::health::ready))
        // --- employees (RBAC-guarded, ARCHITECTURE.md §3.4) ---
        .route(
            "/employees",
            get(handlers::employees::list).post(handlers::employees::hire),
        )
        .route("/employees/{employee_id}", get(handlers::employees::get))
        .route(
            "/employees/{employee_id}/terminate",
            post(handlers::employees::terminate),
        )
        // --- attendance (RBAC-guarded, ARCHITECTURE.md §3.4) ---
        .route(
            "/attendance",
            get(handlers::attendance::list).post(handlers::attendance::record),
        )
        .route(
            "/attendance/{attendance_id}",
            get(handlers::attendance::get),
        )
        // --- leave requests (RBAC-guarded, ARCHITECTURE.md §3.4) ---
        .route(
            "/leave",
            get(handlers::leave::list).post(handlers::leave::request),
        )
        .route("/leave/{leave_id}", get(handlers::leave::get))
        .route("/leave/{leave_id}/approve", post(handlers::leave::approve))
        .route("/leave/{leave_id}/reject", post(handlers::leave::reject));
    // HRM slices complete: employees, attendance, leave.

    Router::new()
        .nest("/api/v1/hrm", hrm_routes)
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
