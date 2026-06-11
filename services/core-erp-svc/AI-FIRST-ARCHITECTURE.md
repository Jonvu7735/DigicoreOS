# core-erp-svc – AI-First quick reference

Condensed pointer. Source of truth: `docs/AI-FIRST-ARCHITECTURE.md`,
`docs/SERVICE-core-erp-svc.md`, `docs/ARCHITECTURE.md`, `docs/API-GATEWAY.md`,
`docs/EVENTS.md`, `docs/DATA-STRATEGY.md`, `docs/OBSERVABILITY.md`.

## Layout (must not drift)
- `src/bootstrap/` – config (env) + wiring (DI, AppState, Router).
- `src/api/http/` – Axum routes, handlers, DTOs, middleware. Calls domain only.
- `src/domain/` – pure business logic (orders, inventory, products, invoices,
  payments). **No axum/sqlx/nats/tracing imports here.**
- `src/infra/` – implementations of domain ports: db (sqlx/Postgres, schema
  `erp_core_svc`), messaging (NATS), time.
- `src/utils/` – logging bootstrap, id generation.

## Contracts
- HTTP under gateway prefix `/api/v1/erp/...` (API-GATEWAY.md §4).
- Events from the shared `event-models` crate (`ErpEvent`), published via the
  outbox pattern (DATA-STRATEGY.md §3.2).
- One Postgres schema: `erp_core_svc`. No cross-schema access.
