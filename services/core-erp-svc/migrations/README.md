# erp_core_svc migrations (sqlx)

Versioned SQL migrations for the `erp_core_svc` schema, applied at startup by
`infra/db/postgres.rs::run_migrations`. One schema per service; no table may
reference another service's schema (DATA-STRATEGY.md §3).

- `0001_init.sql` — schema + `outbox_events` (outbox pattern). Domain tables
  (products, orders, order_items, inventory, invoices, payments) land with
  their respective domain slices.
