# reporting_svc migrations (sqlx)

Versioned SQL migrations for the `reporting_svc` schema, applied at startup by
`infra/db/postgres.rs::run_migrations`. One schema per service; no table may
reference another service's schema (DATA-STRATEGY.md §3).

- `0001_init.sql` — schema + `outbox_events` (reporting also produces events) +
  `processed_events` (consumer idempotency). Fact/dimension/snapshot tables land
  with their respective read-model slices.
