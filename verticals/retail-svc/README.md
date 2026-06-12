# retail-svc

The platform's **second vertical** (sector module) — a retail loyalty programme.
Like [`trade-export-svc`](../trade-export-svc), it extends the platform **without
touching the core** (see [`verticals/README.md`](../README.md)), and exercises the
model against a *different* core event.

## How it plugs in

```
core ERP  ──(platform.erp.order.created)──▶  retail-svc  ──(platform.retail.points.redeemed)──▶  bus
                                                  │
                                          /api/v1/retail/...
```

- **Consumes** the core only via events: on `platform.erp.order.created` it
  accrues loyalty points for the customer (1 point per currency unit), **idempotent
  by `event_id`** (a `processed_events` ledger + the accrual in one transaction).
- **Owns** its bounded context: schema `retail_svc`, the `loyalty_accounts`
  aggregate (balance, lifetime spend, derived `BRONZE`/`SILVER`/`GOLD` tier), and
  its own event `platform.retail.points.redeemed` (defined locally).
- **Reuses** the shared platform libraries; **authorizes by role** (`OWNER`/
  `ADMIN`/`MANAGER` redeem; any role reads) rather than extending the core RBAC
  catalogue.

## API (`/api/v1/retail`)

| Method & path | Purpose |
|---|---|
| `GET /loyalty` | List loyalty accounts (tenant-scoped, paginated) |
| `GET /loyalty/{customer_id}` | One customer's balance + tier |
| `POST /loyalty/{customer_id}/redeem` | Redeem points → emits `PointsRedeemed` |
| `GET /health`, `GET /ready` | Probes |

## Build, test, run

Standalone Cargo workspace (outside the core workspace), so use `--manifest-path`:

```bash
cargo test   --manifest-path verticals/retail-svc/Cargo.toml
cargo clippy --manifest-path verticals/retail-svc/Cargo.toml --all-targets -- -D warnings

docker build -f deploy/Dockerfile \
  --build-arg SERVICE=retail-svc \
  --build-arg MANIFEST_PATH=verticals/retail-svc/Cargo.toml \
  -t digicore/retail-svc:latest .
```

Kubernetes manifests are in `deploy/k8s/80-retail.yaml`; the gateway route
`/api/v1/retail` and the NetworkPolicy selectors are wired in `50-ingress.yaml`
and `60-network-policy.yaml`.
