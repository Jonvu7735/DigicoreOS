//! Attendance DTOs (`/api/v1/hrm/attendance`).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::attendance::entities::AttendanceRecord;

const DATE_FMT: &str = "%Y-%m-%d";
const TIME_FMT: &str = "%H:%M:%S";

#[derive(Debug, Deserialize)]
pub struct RecordAttendanceRequest {
    pub employee_id: Uuid,
    /// `YYYY-MM-DD`.
    pub date: String,
    /// `HH:MM:SS`.
    #[serde(default)]
    pub check_in: Option<String>,
    /// `HH:MM:SS`.
    #[serde(default)]
    pub check_out: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AttendanceResponse {
    pub id: String,
    pub tenant_id: String,
    pub employee_id: String,
    pub date: String,
    pub check_in: Option<String>,
    pub check_out: Option<String>,
    pub created_at: String,
}

impl From<AttendanceRecord> for AttendanceResponse {
    fn from(r: AttendanceRecord) -> Self {
        Self {
            id: r.id.to_string(),
            tenant_id: r.tenant_id.0,
            employee_id: r.employee_id.to_string(),
            date: r.date.format(DATE_FMT).to_string(),
            check_in: r.check_in.map(|t| t.format(TIME_FMT).to_string()),
            check_out: r.check_out.map(|t| t.format(TIME_FMT).to_string()),
            created_at: r.created_at.to_rfc3339(),
        }
    }
}
