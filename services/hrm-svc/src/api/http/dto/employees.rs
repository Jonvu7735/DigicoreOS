//! Employee DTOs (`/api/v1/hrm/employees`).

use serde::{Deserialize, Serialize};

use crate::domain::employees::entities::Employee;

#[derive(Debug, Deserialize)]
pub struct HireEmployeeRequest {
    pub full_name: String,
    pub position: String,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TerminateEmployeeRequest {
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EmployeeResponse {
    pub id: String,
    pub tenant_id: String,
    pub full_name: String,
    pub position: String,
    pub email: Option<String>,
    pub status: String,
    pub created_at: String,
}

impl From<Employee> for EmployeeResponse {
    fn from(e: Employee) -> Self {
        Self {
            id: e.id.to_string(),
            tenant_id: e.tenant_id.0,
            full_name: e.full_name,
            position: e.position,
            email: e.email,
            status: e.status.as_str().to_string(),
            created_at: e.created_at.to_rfc3339(),
        }
    }
}
