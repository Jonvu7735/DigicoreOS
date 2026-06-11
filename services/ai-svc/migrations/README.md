# ai_svc migrations (sqlx)

Versioned SQL migrations for the `ai_svc` schema, applied at startup by
`infra/db/postgres.rs::run_migrations`. One schema per service; no table may
reference another service's schema (DATA-STRATEGY.md §3).

- `0001_init.sql` — schema + `outbox_events` (ai-svc produces AiInsightGenerated)
  + `processed_events` (consumer idempotency). Insight/embedding/model-config
  tables land with their respective slices.
