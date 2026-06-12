# Observability wiring

Makes `OBSERVABILITY.md §7` (Dashboard & Alerting) concrete. The services already
emit Prometheus metrics on `GET /metrics` and W3C-propagating OTLP traces (via
`platform-observability`); this directory wires Prometheus + Grafana + alerts to
them. **Deploy-only — no application changes.**

| File | What it is |
|---|---|
| `prometheus-scrape.yaml` | A scrape job (ConfigMap) that discovers every service's `/metrics` via the Kubernetes API and adds a clean `service` label |
| `alerts.yaml` | `PrometheusRule` — service-down, HTTP error-rate/latency, auth failure spikes, event-consumer failures |
| `dashboards/digicore-overview.json` | Grafana dashboard: service availability, HTTP request/error/latency, auth rates, event throughput |

## Wire it up

**Scraping** — point Prometheus at the platform:
- *Vanilla Prometheus*: merge the `scrape_configs` from `prometheus-scrape.yaml` into
  your `prometheus.yml` (it needs RBAC to list endpoints in the `digicore` namespace).
- *Prometheus Operator*: the same selector as a `ServiceMonitor` (endpoints in
  `digicore`, path `/metrics`).

**Alerts** — `kubectl apply -f alerts.yaml` (Operator), or copy `spec.groups` into a
vanilla `rule_files` target. Adjust the `release` label to match your operator's
`ruleSelector`.

**Dashboard** — import `dashboards/digicore-overview.json` into Grafana and pick your
Prometheus data source. UID `digicore-overview`.

## Metrics it builds on

Emitted by every service (`platform-observability` + the services): `up` (from
scraping); the standard HTTP metrics (`OBSERVABILITY.md §4.3`)
`http_requests_total{service,method,path,status}` and
`http_request_duration_seconds_bucket{service,method,path}` (via the shared
`track_http_metrics` middleware); `auth_login_success_total` /
`auth_login_failed_total`, `auth_refresh_success_total` /
`auth_refresh_failed_total`, `auth_register_success_total`;
`events_published_total`, `events_consumed_total`, `events_consumed_failed_total`.

> **Tracing**: spans export to OTLP (Jaeger/Tempo) when
> `OTEL_EXPORTER_OTLP_ENDPOINT` is set; `platform-observability` propagates the W3C
> `traceparent` across services.
