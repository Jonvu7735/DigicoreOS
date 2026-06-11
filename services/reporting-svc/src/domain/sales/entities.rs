//! Sales summary read model (maps to `reporting_svc.sales_summary`).

use chrono::{DateTime, Utc};

use crate::domain::shared::types::{Money, TenantId};

/// Per-tenant rollup of paid orders. A projection of the `OrderPaid` event
/// stream (eventually consistent).
#[derive(Debug, Clone)]
pub struct SalesSummary {
    pub tenant_id: TenantId,
    /// Sum of `amount_paid` across all processed `OrderPaid` events.
    pub total_paid: Money,
    pub payment_count: i64,
    /// When the projection was last updated; `None` if no events yet.
    pub updated_at: Option<DateTime<Utc>>,
}
