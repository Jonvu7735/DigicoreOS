# DigicoreOS – Core Monorepo

AI-first, event-driven, multi-tenant B2B SaaS core (sector-agnostic ERP/CRM/HRM/Reporting/AI).

## Layout

```text
libs/
  event-models/            # Shared business-event contracts (source of truth: EVENTS.md)
  platform-observability/  # JSON tracing + Prometheus bootstrap (OBSERVABILITY.md)
services/
  auth-svc/                # Identity provider: auth, user, tenant, RBAC, JWT      [Phase 1]
  core-erp-svc/            # Orders, products, inventory, invoices, payments       [Phase 3]
  crm-svc/                 # Customers, contacts, deals, pipeline, activities      [Phase 4]
  hrm-svc/                 # Employees, attendance, leave                          [Phase 4]
  reporting-svc/           # Fact/dim tables, snapshots, dashboard & export API    [Phase 5]
  ai-svc/                  # LLM abstraction, RAG, embeddings, insights            [Phase 5]
deploy/
  docker-compose.dev.yml   # Local Postgres + NATS for development
docs/                      # Governing docs (ARCHITECTURE, AUTH-FLOW, SECURITY, EVENTS, ...)
verticals/                 # Sector modules (retail, trade-export, ...) – LATER.
                           # Consume core via public APIs/events ONLY. Never core deps.
```

## Rules (binding for humans and AI agents)

- Read `AI-FIRST-ARCHITECTURE.md` before touching any service; per-service docs
  (`SERVICE-auth-svc.md`, ...) before touching that service.
- Per-service layout: `bootstrap/ | api/ | domain/ | infra/ | utils/`.
- Dependency rules: `domain` imports no HTTP/DB/messaging; `infra` only
  implements `domain` ports; `api` only calls `domain` services.
- Events are defined ONLY in `libs/event-models` and documented in `EVENTS.md`.
- One Postgres schema per service (`auth_svc`, `erp_core_svc`, `crm_svc`,
  `hrm_svc`, `reporting_svc`). No cross-schema joins, ever.
- Every service exposes `/api/v1/<svc>/health`, `/api/v1/<svc>/ready`,
  `/metrics`, and logs JSON per `OBSERVABILITY.md`.

## Quick start

```bash
docker compose -f deploy/docker-compose.dev.yml up -d   # Postgres + NATS
bash scripts/gen-dev-jwt-keys.sh                        # dev RS256 keys -> .dev/ (gitignored)
cargo run -p auth-svc                                   # http://localhost:8081 (applies migrations)
curl localhost:8081/api/v1/auth/health
```

Auth flows (`/api/v1/auth/login|refresh|logout`) are live in Phase 1.2; sign-up
(`/register`) and user/tenant management land in Phase 1.3.
