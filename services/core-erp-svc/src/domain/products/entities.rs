//! Product entity (maps to `erp_core_svc.products`).

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::shared::types::{Money, TenantId};

#[derive(Debug, Clone)]
pub struct Product {
    pub id: Uuid,
    pub tenant_id: TenantId,
    /// Stock-keeping unit, unique per tenant.
    pub sku: String,
    pub name: String,
    /// Unit price in minor currency units.
    pub price: Money,
    /// ISO 4217 3-letter code.
    pub currency: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}
