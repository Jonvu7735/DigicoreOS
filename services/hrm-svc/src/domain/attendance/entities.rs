//! Attendance record entity (maps to `hrm_svc.attendance`).

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use uuid::Uuid;

use crate::domain::shared::types::TenantId;

#[derive(Debug, Clone)]
pub struct AttendanceRecord {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub employee_id: Uuid,
    pub date: NaiveDate,
    pub check_in: Option<NaiveTime>,
    pub check_out: Option<NaiveTime>,
    pub created_at: DateTime<Utc>,
}
