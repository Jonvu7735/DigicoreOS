# Database migrations & rollback strategy

Each service owns its schema and applies versioned SQL migrations at startup via
`sqlx::migrate!` (`infra/db/postgres.rs::run_migrations`), tracked in that
schema's `_sqlx_migrations` table. Migrations are **forward-only**: `sqlx`'s
embedded runner has no automatic "down" step, and rolling a schema backwards in
place is unsafe while old and new code may both be running during a rollout.

The platform therefore uses **expand / contract** plus **backup-based recovery**
rather than down-migrations.

## Expand / contract (safe rollouts)

Make every schema change backward-compatible across at least one release so a
new migration never breaks the currently-running (old) code, and a rollback of
the *code* never breaks against the already-migrated schema:

1. **Expand** — additive only: new nullable column / new table / new index
   (`CREATE INDEX CONCURRENTLY` for large tables). Old code ignores it.
2. **Migrate data** — backfill in a separate step if needed (idempotent).
3. **Deploy code** that writes/reads the new shape (dual-write if replacing a column).
4. **Contract** — only in a *later* release, once no running code uses the old
   shape: drop the old column/table.

Avoid in a single release: renaming or dropping a column still read by the live
version, or `NOT NULL` without a default on a populated table.

## Rolling back

- **Code**: roll back to the previous image. Because migrations are expand-only,
  the older code still runs against the newer schema.
- **Schema**: do NOT hand-edit `_sqlx_migrations`. If a migration is genuinely
  bad, roll forward with a new corrective migration. For data corruption, restore
  from backup:
  - single-pod Postgres: the daily logical dump (`deploy/k8s/20-postgres.yaml`
    CronJob) — `gunzip -c <dump>.sql.gz | psql`.
  - CloudNativePG overlay: point-in-time recovery from WAL archive
    (`deploy/k8s/ha-postgres/`), recovering to just before the bad change.

## Pre-deploy guardrail

Take a backup immediately before a deploy that includes a migration (the CD
workflow's rollout step is the natural place to trigger an on-demand backup),
so the RPO for a migration-induced problem is ~0 rather than up to a day.
