-- CRM activities (crm_svc.activities). Tenant-scoped, references customers.
CREATE TABLE IF NOT EXISTS activities (
    id          UUID PRIMARY KEY,
    tenant_id   TEXT        NOT NULL,
    customer_id UUID        NOT NULL REFERENCES customers (id) ON DELETE CASCADE,
    kind        TEXT        NOT NULL,          -- CALL | EMAIL | MEETING | TASK
    subject     TEXT        NOT NULL,
    notes       TEXT,
    occurred_at TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS activities_tenant_idx ON activities (tenant_id);
CREATE INDEX IF NOT EXISTS activities_customer_idx ON activities (customer_id, occurred_at DESC);
