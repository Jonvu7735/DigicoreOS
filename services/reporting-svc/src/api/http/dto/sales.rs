//! Sales dashboard DTOs (`/api/v1/reporting/sales-summary`).

use serde::Serialize;

use crate::domain::sales::entities::SalesSummary;

#[derive(Debug, Serialize)]
pub struct SalesSummaryResponse {
    pub tenant_id: String,
    /// Sum of paid amounts, minor currency units.
    pub total_paid: i64,
    pub payment_count: i64,
    /// `None` until the first `OrderPaid` is projected.
    pub updated_at: Option<String>,
}

impl From<SalesSummary> for SalesSummaryResponse {
    fn from(s: SalesSummary) -> Self {
        Self {
            tenant_id: s.tenant_id.0,
            total_paid: s.total_paid.0,
            payment_count: s.payment_count,
            updated_at: s.updated_at.map(|t| t.to_rfc3339()),
        }
    }
}
