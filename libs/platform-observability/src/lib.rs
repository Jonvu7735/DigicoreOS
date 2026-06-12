//! # platform-observability
//!
//! Standard observability bootstrap for every DigicoreOS service.
//! Governing doc: `OBSERVABILITY.md`.
//!
//! Provides:
//! - [`init_tracing`]: JSON structured logging (`tracing` + `tracing-subscriber`)
//!   plus, when `OTEL_EXPORTER_OTLP_ENDPOINT` is set, a W3C-propagating OTLP span
//!   exporter (OBSERVABILITY.md §5). Returns a [`TracingGuard`] that flushes
//!   pending spans on drop.
//! - [`install_prometheus`]: a Prometheus recorder; services render it on `GET /metrics`.
//! - [`set_parent_from_w3c`]: link a span to an upstream trace via `traceparent`.
//!
//! Log fields contract (`OBSERVABILITY.md` §3.2): `timestamp`, `level`, `service`,
//! `env`, `tenant_id`, `user_id`, `trace_id`, `span_id`, `message`.

pub mod http_metrics;
pub use http_metrics::track_http_metrics;

use std::collections::HashMap;

use metrics_exporter_prometheus::PrometheusBuilder;
pub use metrics_exporter_prometheus::PrometheusHandle;
use opentelemetry::propagation::Extractor;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::TracerProvider;
use opentelemetry_sdk::Resource;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// Holds the OTLP tracer provider (if any) and flushes spans when dropped.
/// Keep it alive for the lifetime of the process.
#[must_use = "drop flushes spans; bind it to a variable for the program's lifetime"]
pub struct TracingGuard {
    provider: Option<TracerProvider>,
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.provider.take() {
            if let Err(error) = provider.shutdown() {
                eprintln!("tracer provider shutdown error: {error}");
            }
        }
    }
}

/// Initialize JSON logging and (optionally) OTLP tracing for a service. Call once,
/// first thing in `main`. Log level is controlled by `RUST_LOG` (defaults to `info`).
pub fn init_tracing(service: &str, env: &str) -> TracingGuard {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .flatten_event(true)
        .with_current_span(true)
        .with_span_list(true);

    // W3C `traceparent` propagation in/out (cheap & correct even without an exporter).
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

    let provider = otlp_provider(service);
    let otel_layer = provider
        .as_ref()
        .map(|p| tracing_opentelemetry::layer().with_tracer(p.tracer(service.to_string())));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    tracing::info!(
        service,
        env,
        otlp = provider.is_some(),
        "observability: tracing initialized"
    );
    TracingGuard { provider }
}

/// Build an OTLP tracer provider when `OTEL_EXPORTER_OTLP_ENDPOINT` is configured.
fn otlp_provider(service: &str) -> Option<TracerProvider> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .ok()
        .filter(|v| !v.is_empty())?;

    match opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
    {
        Ok(exporter) => {
            let provider = TracerProvider::builder()
                .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
                .with_resource(Resource::new(vec![KeyValue::new(
                    "service.name",
                    service.to_string(),
                )]))
                .build();
            opentelemetry::global::set_tracer_provider(provider.clone());
            Some(provider)
        }
        Err(error) => {
            // Subscriber not installed yet, so log to stderr directly.
            eprintln!("OTLP exporter init failed ({error}); continuing without OTLP");
            None
        }
    }
}

/// Carrier over the two W3C trace headers.
struct W3cCarrier(HashMap<String, String>);
impl Extractor for W3cCarrier {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }
    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(String::as_str).collect()
    }
}

/// Make `span` a child of the upstream trace described by `traceparent` /
/// `tracestate` (OBSERVABILITY.md §5.3). No-op without a `traceparent`.
pub fn set_parent_from_w3c(
    span: &tracing::Span,
    traceparent: Option<&str>,
    tracestate: Option<&str>,
) {
    let Some(traceparent) = traceparent else {
        return;
    };
    let mut map = HashMap::new();
    map.insert("traceparent".to_string(), traceparent.to_string());
    if let Some(tracestate) = tracestate {
        map.insert("tracestate".to_string(), tracestate.to_string());
    }
    let carrier = W3cCarrier(map);
    let parent = opentelemetry::global::get_text_map_propagator(|p| p.extract(&carrier));
    span.set_parent(parent);
}

/// Install the global Prometheus metrics recorder and return its handle.
/// Services expose `handle.render()` on `GET /metrics` (OBSERVABILITY.md §4.2).
pub fn install_prometheus() -> anyhow::Result<PrometheusHandle> {
    // Render `http_request_duration_seconds` as a Prometheus histogram (`_bucket`
    // series) so dashboards can compute quantiles (OBSERVABILITY.md §4.3); without
    // explicit buckets this exporter would emit a summary (no histogram_quantile).
    const DURATION_BUCKETS: &[f64] = &[
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ];
    let handle = PrometheusBuilder::new()
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Full("http_request_duration_seconds".to_string()),
            DURATION_BUCKETS,
        )
        .map_err(|e| anyhow::anyhow!("failed to set histogram buckets: {e}"))?
        .install_recorder()
        .map_err(|e| anyhow::anyhow!("failed to install prometheus recorder: {e}"))?;
    Ok(handle)
}
