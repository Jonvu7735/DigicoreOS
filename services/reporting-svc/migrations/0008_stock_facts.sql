-- Inventory read model (reporting_svc.stock_facts): stock-on-hand per
-- (product, warehouse), projected from the StockAdjusted event stream
-- (see infra/db/inventory_repo_pg.rs).
--
-- StockAdjusted carries a SIGNED delta, so application is additive and NOT
-- naturally idempotent; each event_id is claimed in processed_events (in the
-- same transaction as the running-sum update), so an at-least-once re-delivery
-- never double-counts.
CREATE TABLE IF NOT EXISTS stock_facts (
    tenant_id    TEXT        NOT NULL,
    product_id   TEXT        NOT NULL,
    warehouse_id TEXT        NOT NULL,
    quantity     BIGINT      NOT NULL,   -- running sum of deltas (units)
    updated_at   TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (tenant_id, product_id, warehouse_id)
);
