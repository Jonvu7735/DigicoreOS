-- Orders read model (reporting_svc.order_facts): one row per order, projected
-- from the OrderCreated event stream (see infra/db/orders_repo_pg.rs).
-- Idempotent on order_id (ON CONFLICT DO NOTHING), so at-least-once re-delivery
-- of the same OrderCreated is a no-op.
CREATE TABLE IF NOT EXISTS order_facts (
    order_id     TEXT PRIMARY KEY,
    tenant_id    TEXT        NOT NULL,
    customer_id  TEXT        NOT NULL,
    total_amount BIGINT      NOT NULL,   -- minor currency units
    currency     TEXT        NOT NULL,
    status       TEXT        NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS order_facts_tenant_idx ON order_facts (tenant_id, created_at DESC);
