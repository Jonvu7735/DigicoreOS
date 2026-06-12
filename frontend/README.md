# DigicoreOS — Web frontend

Vite + React + TypeScript SPA for the DigicoreOS platform. It talks to the
services through a **fully-typed** client (`openapi-fetch`) whose types are
generated from the single source of truth, `docs/openapi.yaml` — so the UI can
never drift from the API contract (CI enforces it).

## Develop

```bash
npm install
npm run dev        # http://localhost:5173
```

The dev server proxies `/api/v1/<service>` to each service's local port (see
`vite.config.ts`), since there is no API gateway yet. Start the backend first:

```bash
# from the repo root
docker compose -f deploy/docker-compose.dev.yml up -d   # postgres + nats
cargo run -p auth-svc                                    # :8081 (and others as needed)
```

Then sign in at `/login` with a tenant user (create one via `POST /api/v1/auth/register`).

## Scripts

| | |
|---|---|
| `npm run dev` | dev server (HMR) |
| `npm run build` | typecheck (`tsc -b`) + production build |
| `npm run lint` | ESLint |
| `npm run generate` | regenerate `src/api/schema.d.ts` from `docs/openapi.yaml` |

After changing `docs/openapi.yaml`, run `npm run generate` and commit the result
(the `frontend (web)` CI job fails on drift).

## Layout

- `src/api/` — generated schema + typed client factory
- `src/auth/` — token/auth context (JWT claims decoded client-side for display)
- `src/pages/`, `src/components/` — screens + routing guards
