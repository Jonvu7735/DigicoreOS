-- CRM contacts (crm_svc.contacts). Tenant-scoped, references customers.
CREATE TABLE IF NOT EXISTS contacts (
    id          UUID PRIMARY KEY,
    tenant_id   TEXT        NOT NULL,
    customer_id UUID        NOT NULL REFERENCES customers (id) ON DELETE CASCADE,
    name        TEXT        NOT NULL,
    email       TEXT,
    phone       TEXT,
    title       TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS contacts_tenant_idx ON contacts (tenant_id);
CREATE INDEX IF NOT EXISTS contacts_customer_idx ON contacts (customer_id);
