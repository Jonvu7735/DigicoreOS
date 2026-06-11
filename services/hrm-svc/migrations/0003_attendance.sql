-- HRM attendance (hrm_svc.attendance). Tenant-scoped, references employees.
CREATE TABLE IF NOT EXISTS attendance (
    id          UUID PRIMARY KEY,
    tenant_id   TEXT        NOT NULL,
    employee_id UUID        NOT NULL REFERENCES employees (id) ON DELETE CASCADE,
    date        DATE        NOT NULL,
    check_in    TIME,
    check_out   TIME,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS attendance_tenant_idx ON attendance (tenant_id);
CREATE INDEX IF NOT EXISTS attendance_employee_idx ON attendance (employee_id, date);
