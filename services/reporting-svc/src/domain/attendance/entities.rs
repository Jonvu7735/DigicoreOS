//! Attendance read-model entities (map to `reporting_svc.attendance_facts`).

use crate::domain::shared::types::TenantId;

/// The fields needed to project an attendance row (from an `AttendanceRecorded`).
#[derive(Debug, Clone)]
pub struct NewAttendanceFact {
    pub tenant_id: TenantId,
    pub employee_id: String,
    pub work_date: String,
    pub check_in: Option<String>,
    pub check_out: Option<String>,
}

/// Per-tenant attendance rollup for the HRM summary.
#[derive(Debug, Clone)]
pub struct AttendanceSummary {
    /// Total attendance records (employee-days).
    pub record_count: i64,
    /// Distinct employees with at least one attendance record.
    pub present_employees: i64,
}
