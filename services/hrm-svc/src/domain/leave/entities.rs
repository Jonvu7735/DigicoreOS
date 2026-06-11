//! Leave request entity + status machine (maps to `hrm_svc.leave_requests`).

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::domain::shared::types::TenantId;

/// Leave request lifecycle. `Approved`/`Rejected` are terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaveStatus {
    Requested,
    Approved,
    Rejected,
}

impl LeaveStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            LeaveStatus::Requested => "REQUESTED",
            LeaveStatus::Approved => "APPROVED",
            LeaveStatus::Rejected => "REJECTED",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "REQUESTED" => Some(LeaveStatus::Requested),
            "APPROVED" => Some(LeaveStatus::Approved),
            "REJECTED" => Some(LeaveStatus::Rejected),
            _ => None,
        }
    }

    /// Only a pending request can be decided: REQUESTED → APPROVED | REJECTED.
    pub fn can_transition_to(self, next: LeaveStatus) -> bool {
        use LeaveStatus::*;
        matches!((self, next), (Requested, Approved) | (Requested, Rejected))
    }
}

#[derive(Debug, Clone)]
pub struct LeaveRequest {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub employee_id: Uuid,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub reason: Option<String>,
    pub status: LeaveStatus,
    pub created_at: DateTime<Utc>,
}
