//! Payment entity (maps to `erp_core_svc.payments`).

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::shared::types::{Money, TenantId};

#[derive(Debug, Clone)]
pub struct Payment {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub order_id: Uuid,
    pub amount: Money,
    /// e.g. `card`, `cash`, `bank_transfer`.
    pub method: String,
    pub created_at: DateTime<Utc>,
}
