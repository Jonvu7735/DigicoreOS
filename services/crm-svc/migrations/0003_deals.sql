-- CRM deals / sales pipeline (crm_svc.deals). Tenant-scoped, references customers.
CREATE TABLE IF NOT EXISTS deals (
    id              UUID PRIMARY KEY,
    tenant_id       TEXT        NOT NULL,
    customer_id     UUID        NOT NULL REFERENCES customers (id) ON DELETE CASCADE,
    title           TEXT        NOT NULL,
    amount_estimate BIGINT      NOT NULL,          -- minor currency units
    stage           TEXT        NOT NULL,          -- LEAD | QUALIFIED | PROPOSAL | WON | LOST
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS deals_tenant_idx ON deals (tenant_id);
CREATE INDEX IF NOT EXISTS deals_customer_idx ON deals (customer_id);
