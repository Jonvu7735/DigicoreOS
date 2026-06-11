//! Leave DTOs (`/api/v1/hrm/leave`).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::leave::entities::LeaveRequest;

const DATE_FMT: &str = "%Y-%m-%d";

#[derive(Debug, Deserialize)]
pub struct RequestLeaveRequest {
    pub employee_id: Uuid,
    /// `YYYY-MM-DD`.
    pub start_date: String,
    /// `YYYY-MM-DD`.
    pub end_date: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LeaveResponse {
    pub id: String,
    pub tenant_id: String,
    pub employee_id: String,
    pub start_date: String,
    pub end_date: String,
    pub reason: Option<String>,
    pub status: String,
    pub created_at: String,
}

impl From<LeaveRequest> for LeaveResponse {
    fn from(r: LeaveRequest) -> Self {
        Self {
            id: r.id.to_string(),
            tenant_id: r.tenant_id.0,
            employee_id: r.employee_id.to_string(),
            start_date: r.start_date.format(DATE_FMT).to_string(),
            end_date: r.end_date.format(DATE_FMT).to_string(),
            reason: r.reason,
            status: r.status.as_str().to_string(),
            created_at: r.created_at.to_rfc3339(),
        }
    }
}
