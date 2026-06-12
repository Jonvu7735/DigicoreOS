-- Customers read model (reporting_svc.customer_facts): one row per customer,
-- projected from the CustomerCreated event stream (see infra/db/customers_repo_pg.rs).
-- Idempotent on customer_id (ON CONFLICT DO NOTHING), so at-least-once re-delivery
-- of the same CustomerCreated is a no-op.
CREATE TABLE IF NOT EXISTS customer_facts (
    customer_id TEXT PRIMARY KEY,
    tenant_id   TEXT        NOT NULL,
    name        TEXT        NOT NULL,
    email       TEXT,                  -- nullable (CustomerCreated.email is optional)
    segment     TEXT,                  -- nullable (e.g. VIP / SMB / ...)
    created_at  TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS customer_facts_tenant_idx ON customer_facts (tenant_id, created_at DESC);
