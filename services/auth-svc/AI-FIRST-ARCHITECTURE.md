# auth-svc – AI-First quick reference

Condensed pointer file. The source of truth lives at the repo root / project docs:
`AI-FIRST-ARCHITECTURE.md`, `SERVICE-auth-svc.md`, `AUTH-FLOW.md`, `SECURITY.md`,
`API-GATEWAY.md`, `EVENTS.md`, `OBSERVABILITY.md`, `DATA-STRATEGY.md`.

## Layout (must not drift)
- `src/bootstrap/` – config (env) + wiring (DI, AppState, Router).
- `src/api/http/` – Axum routes, handlers, DTOs, middleware. Calls domain only.
- `src/domain/` – pure business logic: `shared/` (error, types) + `identity/`
  (entities, ports, services). **No axum/sqlx/nats/tracing imports here.**
- `src/infra/` – implementations of domain ports: db (sqlx/Postgres, schema `auth_svc`),
  messaging (NATS), security (jwt, password), time (clock).
- `src/utils/` – logging bootstrap, id generation.

## Dependency rules
1. `domain` must not depend on `api`, `infra`, `utils`.
2. `infra` only implements traits defined in `domain`.
3. `api` calls domain services/ports; never touches DB/messaging directly.

## Contracts
- HTTP under gateway prefix `/api/v1/auth/...` (API-GATEWAY.md).
- Events from the shared `event-models` crate, published via the
  `EventPublisher` port after DB commit (outbox pattern – DATA-STRATEGY.md §3.2).
