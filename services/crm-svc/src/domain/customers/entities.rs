//! Customer entity (maps to `crm_svc.customers`).

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::shared::types::TenantId;

#[derive(Debug, Clone)]
pub struct Customer {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    /// Free-form classification (e.g. `vip`, `smb`); semantics owned by the tenant.
    pub segment: Option<String>,
    pub created_at: DateTime<Utc>,
}
