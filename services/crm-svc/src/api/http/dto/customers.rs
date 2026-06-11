//! Customer DTOs (`/api/v1/crm/customers`).

use serde::{Deserialize, Serialize};

use crate::domain::customers::entities::Customer;

#[derive(Debug, Deserialize)]
pub struct CreateCustomerRequest {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub segment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCustomerRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub segment: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CustomerResponse {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub segment: Option<String>,
    pub created_at: String,
}

impl From<Customer> for CustomerResponse {
    fn from(c: Customer) -> Self {
        Self {
            id: c.id.to_string(),
            tenant_id: c.tenant_id.0,
            name: c.name,
            email: c.email,
            phone: c.phone,
            segment: c.segment,
            created_at: c.created_at.to_rfc3339(),
        }
    }
}
