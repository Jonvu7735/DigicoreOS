-- Export shipments: the vertical's own aggregate.

CREATE TABLE IF NOT EXISTS export_shipments (
    id                  UUID PRIMARY KEY,
    tenant_id           TEXT        NOT NULL,
    -- The ERP order this shipment fulfils, when auto-drafted from `order.paid`.
    order_id            TEXT,
    reference           TEXT        NOT NULL,
    destination_country TEXT        NOT NULL,
    incoterm            TEXT        NOT NULL,
    status              TEXT        NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS export_shipments_tenant_idx ON export_shipments (tenant_id);

-- At most one shipment per (tenant, order): makes the inbound `order.paid`
-- consumer idempotent under at-least-once delivery (defence in depth — the
-- service also checks first).
CREATE UNIQUE INDEX IF NOT EXISTS export_shipments_order_uniq
    ON export_shipments (tenant_id, order_id) WHERE order_id IS NOT NULL;
