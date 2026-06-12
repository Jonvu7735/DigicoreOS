# DigicoreOS – Core Monorepo

AI-first, event-driven, multi-tenant B2B SaaS core (sector-agnostic ERP/CRM/HRM/Reporting/AI).

A Rust workspace of six microservices over a shared Postgres (schema-per-service)
and a NATS event bus. Every write that produces an event commits state + event in
one transaction (transactional outbox); analytics and AI are built by *consuming*
those events.

## Services

| Service          | Port | Schema           | Bounded contexts                                   | Publishes |
|------------------|------|------------------|----------------------------------------------------|-----------|
| `auth-svc`       | 8081 | `auth_svc`       | identity: login/refresh/logout, users, tenants, RBAC | `TenantCreated`, `UserRegistered`, `UserUpdated` |
| `core-erp-svc`   | 8082 | `erp_core_svc`   | products, orders, payments, inventory, invoices    | `OrderCreated`, `OrderStatusChanged`, `OrderPaid`, `StockAdjusted`, `InvoiceIssued` |
| `crm-svc`        | 8083 | `crm_svc`        | customers, deals (+pipeline), contacts, activities | `CustomerCreated`, `CustomerUpdated`, `DealCreated`, `DealStageChanged` |
| `hrm-svc`        | 8084 | `hrm_svc`        | employees, attendance, leave                       | `EmployeeHired`, `EmployeeTerminated`, `AttendanceRecorded` |
| `reporting-svc`  | 8085 | `reporting_svc`  | event **consumer** → read models, dashboards, snapshots | `ReportSnapshotCreated` |
| `ai-svc`         | 8086 | `ai_svc`         | insight engine (LLM behind a port), event **consumer** | `AiInsightGenerated` |

All NATS subjects follow `platform.<domain>.<entity>.<action>` (see `docs/EVENTS.md`).

### The end-to-end event loop

```
erp OrderPaid ──▶ reporting sales-summary (read model) ──▶ POST /snapshots ──▶ ReportSnapshotCreated
                                                                                      │
                                                                            ai-svc consumes ──▶ AiInsightGenerated
```

## Layout

```text
libs/
  event-models/            # Shared business-event contracts (source of truth: EVENTS.md)
  platform-auth/           # RBAC matrix + JWT (RS256) verification + AuthContext
  platform-outbox/         # Transactional outbox: insert_outbox + relay (producer side)
  platform-events/         # NATS consumer + InboundEventHandler port (consumer side)
  platform-observability/  # JSON tracing + Prometheus bootstrap (OBSERVABILITY.md)
services/                  # one crate per microservice (see table above)
deploy/
  docker-compose.dev.yml   # Local Postgres + NATS for development
scripts/
  gen-dev-jwt-keys.sh      # Generate dev RS256 key pair into ./.dev/ (gitignored)
docs/                      # Governing docs (ARCHITECTURE, AUTH-FLOW, SECURITY, EVENTS, ...)
verticals/                 # Sector modules (retail, trade-export, ...) – LATER.
                           # Consume core via public APIs/events ONLY. Never core deps.
```

## Architecture rules (binding for humans and AI agents)

- **Hexagonal layout** per service: `bootstrap/ | api/ | domain/ | infra/ | utils/`.
  `domain` imports no HTTP/DB/messaging; `infra` only implements `domain` ports;
  `api` only calls `domain` services.
- **Events** are defined ONLY in `libs/event-models` and documented in `EVENTS.md`.
  A producer writes them via `platform-outbox`; a consumer reads them via
  `platform-events`. Decoding stays in the service (it owns the contracts).
- **One Postgres schema per service** (`auth_svc`, `erp_core_svc`, `crm_svc`,
  `hrm_svc`, `reporting_svc`, `ai_svc`). No cross-schema joins, ever. Each
  service owns its `outbox_events`; consumers dedupe via `processed_events`
  (at-least-once delivery).
- **Auth**: `auth-svc` issues RS256 JWTs; every other service only *verifies*
  them (`platform-auth`) and enforces the shared RBAC matrix (5 roles, 41
  permissions — `platform_auth::rbac`). The matrix is human-governed (SECURITY.md).
- Every service exposes `/api/v1/<svc>/health`, `/api/v1/<svc>/ready`,
  `/metrics`, and logs JSON per `OBSERVABILITY.md`.
- Read `AI-FIRST-ARCHITECTURE.md` before touching any service.

## Quick start (local dev)

```bash
docker compose -f deploy/docker-compose.dev.yml up -d   # Postgres + NATS
bash scripts/gen-dev-jwt-keys.sh                        # dev RS256 keys -> .dev/ (gitignored)

# Each service applies its own migrations on boot and reads .dev/jwt_public.pem.
cargo run -p auth-svc        # :8081  /api/v1/auth
cargo run -p core-erp-svc    # :8082  /api/v1/erp
cargo run -p crm-svc         # :8083  /api/v1/crm
cargo run -p hrm-svc         # :8084  /api/v1/hrm
cargo run -p reporting-svc   # :8085  /api/v1/reporting
cargo run -p ai-svc          # :8086  /api/v1/ai

curl localhost:8081/api/v1/auth/health
```

Services run without NATS (events accumulate in `outbox_events`; consumers idle)
and connect lazily to Postgres, so individual services boot for local work even
if the bus is down.

## Build, lint, test

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI (`.github/workflows/ci.yml`) runs exactly these three on every PR. Unit tests
are DB-free (domain logic against fake ports + an RS256 round-trip); DB-backed
integration tests are gated behind `TEST_DATABASE_URL`. MSRV is Rust 1.96
(`rust-toolchain.toml`).

## Status

All six services are built and implement their full `EVENTS.md` contracts: the
core business services (auth, ERP, CRM, HRM) plus the analytics/AI layer
(reporting consumes events into read models and emits snapshots; ai-svc consumes
events and emits insights). The AI model sits behind a domain port with a
deterministic stub adapter — a real LLM/embedding adapter slots in without
touching domain or API code.
