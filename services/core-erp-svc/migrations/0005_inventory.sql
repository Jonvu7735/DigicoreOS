-- ERP inventory: current stock levels + an append-only adjustment log.
CREATE TABLE IF NOT EXISTS stock_levels (
    tenant_id    TEXT   NOT NULL,
    product_id   UUID   NOT NULL REFERENCES products (id) ON DELETE CASCADE,
    warehouse_id TEXT   NOT NULL,
    quantity     BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (tenant_id, product_id, warehouse_id)
);

CREATE TABLE IF NOT EXISTS stock_adjustments (
    id           UUID PRIMARY KEY,
    tenant_id    TEXT        NOT NULL,
    product_id   UUID        NOT NULL REFERENCES products (id) ON DELETE CASCADE,
    warehouse_id TEXT        NOT NULL,
    delta        BIGINT      NOT NULL,
    reason       TEXT        NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS stock_adjustments_tenant_idx ON stock_adjustments (tenant_id);
