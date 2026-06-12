//! Orders read-model entities (map to `reporting_svc.order_facts`).

use chrono::{DateTime, Utc};

use crate::domain::shared::types::{Money, TenantId};

/// One order, projected from the `OrderCreated` stream (eventually consistent).
#[derive(Debug, Clone)]
pub struct ReportedOrder {
    pub order_id: String,
    pub tenant_id: TenantId,
    pub customer_id: String,
    pub total_amount: Money,
    pub currency: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// The fields needed to project a new order fact (from an `OrderCreated`).
#[derive(Debug, Clone)]
pub struct NewOrderFact {
    pub order_id: String,
    pub tenant_id: TenantId,
    pub customer_id: String,
    pub total_amount: i64,
    pub currency: String,
    pub status: String,
    pub occurred_at: DateTime<Utc>,
}

/// Per-tenant rollup for the overview dashboard.
#[derive(Debug, Clone)]
pub struct OrdersOverview {
    pub order_count: i64,
    pub total_amount: Money,
}
