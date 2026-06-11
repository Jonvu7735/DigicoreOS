-- ERP orders (erp_core_svc.orders). Tenant-scoped order headers.
CREATE TABLE IF NOT EXISTS orders (
    id           UUID PRIMARY KEY,
    tenant_id    TEXT        NOT NULL,
    customer_id  TEXT        NOT NULL,
    total_amount BIGINT      NOT NULL,            -- minor currency units
    currency     TEXT        NOT NULL,
    status       TEXT        NOT NULL,            -- NEW | CONFIRMED | COMPLETED | CANCELLED
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS orders_tenant_idx ON orders (tenant_id);
