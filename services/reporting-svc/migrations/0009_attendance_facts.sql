-- Attendance read model (reporting_svc.attendance_facts): one row per
-- (employee, work_date), projected from the AttendanceRecorded event stream
-- (see infra/db/attendance_repo_pg.rs). Backs the attendance half of the HRM
-- summary; headcount comes from the existing employee_facts projection.
--
-- The natural key (tenant_id, employee_id, work_date) makes application
-- idempotent; a follow-up event for the same day (e.g. carrying check_out)
-- merges via COALESCE without changing the row count.
CREATE TABLE IF NOT EXISTS attendance_facts (
    tenant_id   TEXT NOT NULL,
    employee_id TEXT NOT NULL,
    work_date   TEXT NOT NULL,   -- YYYY-MM-DD (as emitted)
    check_in    TEXT,            -- HH:MM:SS, nullable
    check_out   TEXT,            -- HH:MM:SS, nullable
    PRIMARY KEY (tenant_id, employee_id, work_date)
);
CREATE INDEX IF NOT EXISTS attendance_facts_tenant_date_idx ON attendance_facts (tenant_id, work_date);
