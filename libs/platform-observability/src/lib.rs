//! # platform-observability
//!
//! Standard observability bootstrap for every DigicoreOS service.
//! Governing doc: `OBSERVABILITY.md`.
//!
//! Provides:
//! - [`init_tracing`]: JSON structured logging (`tracing` + `tracing-subscriber`).
//! - [`install_prometheus`]: a Prometheus recorder; services render it on `GET /metrics`.
//!
//! Log fields contract (`OBSERVABILITY.md` §3.2): `timestamp`, `level`, `service`,
//! `env`, `tenant_id`, `user_id`, `trace_id`, `span_id`, `message`.
//! `service`/`env`/`tenant_id`/`user_id` are recorded on the per-request root span
//! (`http.server`) created by each service's HTTP layer; the JSON formatter emits
//! them in the span context of every log line.
//!
//! TODO(Phase 1.4): add `tracing-opentelemetry` bridge + OTLP exporter so
//! `trace_id`/`span_id` are W3C-compatible and propagate over HTTP & NATS.

pub use metrics_exporter_prometheus::PrometheusHandle;
use metrics_exporter_prometheus::PrometheusBuilder;
use tracing_subscriber::EnvFilter;

/// Initialize JSON logging for a service. Call once, first thing in `main`.
///
/// Log level is controlled by `RUST_LOG` (defaults to `info`).
pub fn init_tracing(service: &str, env: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_current_span(true)
        .with_span_list(true)
        .with_env_filter(filter)
        .init();

    tracing::info!(service, env, "observability: JSON tracing initialized");
}

/// Install the global Prometheus metrics recorder and return its handle.
/// Services expose `handle.render()` on `GET /metrics` (OBSERVABILITY.md §4.2).
pub fn install_prometheus() -> anyhow::Result<PrometheusHandle> {
    let handle = PrometheusBuilder::new()
        .install_recorder()
        .map_err(|e| anyhow::anyhow!("failed to install prometheus recorder: {e}"))?;
    Ok(handle)
}
