//! Employees read-model entities (map to `reporting_svc.employee_facts`).

use chrono::{DateTime, Utc};

use crate::domain::shared::types::TenantId;

/// One employee, projected from the `EmployeeHired` stream (eventually
/// consistent).
#[derive(Debug, Clone)]
pub struct ReportedEmployee {
    pub employee_id: String,
    pub tenant_id: TenantId,
    pub full_name: String,
    pub position: String,
    pub created_at: DateTime<Utc>,
}

/// The fields needed to project a new employee fact (from an `EmployeeHired`).
#[derive(Debug, Clone)]
pub struct NewEmployeeFact {
    pub employee_id: String,
    pub tenant_id: TenantId,
    pub full_name: String,
    pub position: String,
    pub occurred_at: DateTime<Utc>,
}
