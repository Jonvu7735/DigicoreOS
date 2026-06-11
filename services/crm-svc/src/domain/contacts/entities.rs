//! Contact entity (maps to `crm_svc.contacts`). A person at a customer org.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::shared::types::TenantId;

#[derive(Debug, Clone)]
pub struct Contact {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub customer_id: Uuid,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    /// Job title / role at the customer.
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
}
