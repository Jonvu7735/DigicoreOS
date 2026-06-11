//! Employee entity + employment status (maps to `hrm_svc.employees`).

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::shared::types::TenantId;

/// Employment lifecycle status. `Terminated` is terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmploymentStatus {
    Active,
    Terminated,
}

impl EmploymentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            EmploymentStatus::Active => "ACTIVE",
            EmploymentStatus::Terminated => "TERMINATED",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "ACTIVE" => Some(EmploymentStatus::Active),
            "TERMINATED" => Some(EmploymentStatus::Terminated),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Employee {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub full_name: String,
    pub position: String,
    pub email: Option<String>,
    pub status: EmploymentStatus,
    pub created_at: DateTime<Utc>,
}
