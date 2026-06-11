-- ERP invoices (erp_core_svc.invoices). Tenant-scoped, references orders.
CREATE TABLE IF NOT EXISTS invoices (
    id         UUID PRIMARY KEY,
    tenant_id  TEXT        NOT NULL,
    order_id   UUID        NOT NULL REFERENCES orders (id) ON DELETE CASCADE,
    amount     BIGINT      NOT NULL,            -- minor currency units
    currency   TEXT        NOT NULL,
    status     TEXT        NOT NULL,            -- ISSUED | CANCELLED
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS invoices_tenant_idx ON invoices (tenant_id);
CREATE INDEX IF NOT EXISTS invoices_order_idx ON invoices (order_id);
