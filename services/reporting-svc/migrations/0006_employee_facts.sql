-- Employees read model (reporting_svc.employee_facts): one row per employee,
-- projected from the EmployeeHired event stream (see infra/db/employees_repo_pg.rs).
-- Idempotent on employee_id (ON CONFLICT DO NOTHING), so at-least-once re-delivery
-- of the same EmployeeHired is a no-op.
CREATE TABLE IF NOT EXISTS employee_facts (
    employee_id TEXT PRIMARY KEY,
    tenant_id   TEXT        NOT NULL,
    full_name   TEXT        NOT NULL,
    position    TEXT        NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS employee_facts_tenant_idx ON employee_facts (tenant_id, created_at DESC);
