//! Route table for core-erp-svc.
//!
//! Business routes live under the gateway prefix `/api/v1/erp/...`
//! (API-GATEWAY.md §4). `/metrics` is exposed at the root for Prometheus.

use axum::routing::{get, post};
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
        )
        // --- orders (RBAC-guarded, API-GATEWAY.md §4.1) ---
        .route(
            "/orders",
            get(handlers::orders::list).post(handlers::orders::create),
        )
        .route("/orders/{order_id}", get(handlers::orders::get))
        .route(
            "/orders/{order_id}/confirm",
            post(handlers::orders::confirm),
        )
        .route(
            "/orders/{order_id}/complete",
            post(handlers::orders::complete),
        )
        .route("/orders/{order_id}/cancel", post(handlers::orders::cancel))
        // --- order payments (API-GATEWAY.md §4.2) ---
        .route(
            "/orders/{order_id}/payments",
            get(handlers::payments::list).post(handlers::payments::record),
        )
        // --- inventory (API-GATEWAY.md §4.3) ---
        .route("/inventory", get(handlers::inventory::list_stock))
        .route(
            "/inventory/adjustments",
            get(handlers::inventory::list_adjustments).post(handlers::inventory::create_adjustment),
        )
        // --- invoices (API-GATEWAY.md §4.5) ---
        .route(
            "/invoices",
            get(handlers::invoices::list).post(handlers::invoices::create),
        )
        .route("/invoices/{invoice_id}", get(handlers::invoices::get))
        .route(
            "/invoices/{invoice_id}/cancel",
            post(handlers::invoices::cancel),
        );
    // ERP core slices complete: products, orders, payments, inventory, invoices.

    Router::new()
        .nest("/api/v1/erp", erp_routes)
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
