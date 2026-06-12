//! Customers read-model entities (map to `reporting_svc.customer_facts`).

use chrono::{DateTime, Utc};

use crate::domain::shared::types::TenantId;

/// One customer, projected from the `CustomerCreated` stream (eventually
/// consistent).
#[derive(Debug, Clone)]
pub struct ReportedCustomer {
    pub customer_id: String,
    pub tenant_id: TenantId,
    pub name: String,
    pub email: Option<String>,
    pub segment: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// The fields needed to project a new customer fact (from a `CustomerCreated`).
#[derive(Debug, Clone)]
pub struct NewCustomerFact {
    pub customer_id: String,
    pub tenant_id: TenantId,
    pub name: String,
    pub email: Option<String>,
    pub segment: Option<String>,
    pub occurred_at: DateTime<Utc>,
}
