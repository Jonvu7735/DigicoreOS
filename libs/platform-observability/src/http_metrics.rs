//! Standard HTTP server metrics (`OBSERVABILITY.md §4.3`): `http_requests_total`
//! and `http_request_duration_seconds`, labelled `service` / `method` / `path` /
//! `status`. The `path` label uses the matched route template (e.g.
//! `/api/v1/erp/orders/{order_id}`) so id values don't explode cardinality.
//!
//! Apply it in a service router (outermost, so it times the whole stack):
//!
//! ```ignore
//! Router::new()
//!     .nest("/api/v1/erp", routes)
//!     .route("/metrics", get(metrics::render))
//!     .layer(axum::middleware::from_fn_with_state(
//!         service_name, platform_observability::track_http_metrics))
//!     .layer(TraceLayer::new_for_http())
//!     .with_state(state)
//! ```

use std::time::Instant;

use axum::extract::{MatchedPath, Request, State};
use axum::middleware::Next;
use axum::response::Response;

/// Axum middleware recording the standard HTTP metrics. `service` is supplied via
/// the layer state (`from_fn_with_state(service_name, track_http_metrics)`).
pub async fn track_http_metrics(
    State(service): State<&'static str>,
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().to_string();
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
        "service" => service,
        "method" => method.clone(),
        "path" => path.clone(),
    )
    .record(start.elapsed().as_secs_f64());
    metrics::counter!(
        "http_requests_total",
        "service" => service,
        "method" => method,
        "path" => path,
        "status" => status,
    )
    .increment(1);

    response
}
