-- ERP order payments (erp_core_svc.payments). Tenant-scoped, references orders.
CREATE TABLE IF NOT EXISTS payments (
    id         UUID PRIMARY KEY,
    tenant_id  TEXT        NOT NULL,
    order_id   UUID        NOT NULL REFERENCES orders (id) ON DELETE CASCADE,
    amount     BIGINT      NOT NULL,             -- minor currency units
    method     TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS payments_order_idx ON payments (order_id);
