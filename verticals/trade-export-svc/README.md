# trade-export-svc

The platform's **first vertical** (sector module) — export shipments for the
trade/export industry. It demonstrates the extension model from
[`verticals/README.md`](../README.md): a vertical extends the platform **without
touching the core**.

## How it plugs in

```
core ERP  ──(platform.erp.order.paid)──▶  trade-export-svc  ──(platform.trade_export.shipment.booked)──▶  bus
                                                  │
                                          /api/v1/trade-export/...
```

- **Consumes** the core only via business events: on `platform.erp.order.paid`
  it drafts an export shipment (idempotently). It never imports a core service
  crate or reads a core schema.
- **Owns** its bounded context: schema `trade_export_svc`, the `export_shipments`
  aggregate, and its own event `platform.trade_export.shipment.booked` (defined
  locally — the shared `event-models` crate stays core-only).
- **Reuses** the shared platform libraries: `platform-auth` (JWT verify),
  `platform-events` (consumer), `platform-outbox` (transactional publish),
  `platform-observability`.
- **Authorizes by role**, not permission: the shared RBAC permission catalogue is
  core policy (seeded by auth-svc) and a vertical must not extend it, so the
  vertical applies its own role policy over the JWT (`OWNER`/`ADMIN`/`MANAGER`
  may write; any role may read).

## API (`/api/v1/trade-export`)

| Method & path | Purpose |
|---|---|
| `GET /shipments` | List shipments (tenant-scoped, paginated) |
| `POST /shipments` | Create a shipment |
| `GET /shipments/{id}` | Shipment detail |
| `POST /shipments/{id}/book` | Book a draft → emits `ShipmentBooked` |
| `GET /health`, `GET /ready` | Probes |

## Build, test, run

It is a **standalone Cargo workspace** (outside the core workspace), so use
`--manifest-path`:

```bash
cargo test  --manifest-path verticals/trade-export-svc/Cargo.toml
cargo clippy --manifest-path verticals/trade-export-svc/Cargo.toml --all-targets -- -D warnings
cargo run   --manifest-path verticals/trade-export-svc/Cargo.toml   # needs DATABASE_URL, JWT key
```

Container image (shared multi-stage `deploy/Dockerfile`, via the `MANIFEST_PATH`
build arg):

```bash
docker build -f deploy/Dockerfile \
  --build-arg SERVICE=trade-export-svc \
  --build-arg MANIFEST_PATH=verticals/trade-export-svc/Cargo.toml \
  -t digicore/trade-export-svc:latest .
```

Kubernetes manifests live in `deploy/k8s/70-trade-export.yaml`; the gateway route
`/api/v1/trade-export` and the NetworkPolicy selectors are wired in
`50-ingress.yaml` and `60-network-policy.yaml`.
