-- HRM employees (hrm_svc.employees). Tenant-scoped.
CREATE TABLE IF NOT EXISTS employees (
    id         UUID PRIMARY KEY,
    tenant_id  TEXT        NOT NULL,
    full_name  TEXT        NOT NULL,
    position   TEXT        NOT NULL,
    email      TEXT,
    status     TEXT        NOT NULL,          -- ACTIVE | TERMINATED
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS employees_tenant_idx ON employees (tenant_id);
