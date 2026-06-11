-- CRM customers (crm_svc.customers). Tenant-scoped.
CREATE TABLE IF NOT EXISTS customers (
    id         UUID PRIMARY KEY,
    tenant_id  TEXT        NOT NULL,
    name       TEXT        NOT NULL,
    email      TEXT,
    phone      TEXT,
    segment    TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS customers_tenant_idx ON customers (tenant_id);
