-- HRM leave requests (hrm_svc.leave_requests). Tenant-scoped, references employees.
CREATE TABLE IF NOT EXISTS leave_requests (
    id          UUID PRIMARY KEY,
    tenant_id   TEXT        NOT NULL,
    employee_id UUID        NOT NULL REFERENCES employees (id) ON DELETE CASCADE,
    start_date  DATE        NOT NULL,
    end_date    DATE        NOT NULL,
    reason      TEXT,
    status      TEXT        NOT NULL,          -- REQUESTED | APPROVED | REJECTED
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS leave_requests_tenant_idx ON leave_requests (tenant_id);
CREATE INDEX IF NOT EXISTS leave_requests_employee_idx ON leave_requests (employee_id);
