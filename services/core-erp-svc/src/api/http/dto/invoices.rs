//! Invoice DTOs (`/api/v1/erp/invoices`).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::invoices::entities::Invoice;

#[derive(Debug, Deserialize)]
pub struct IssueInvoiceRequest {
    pub order_id: Uuid,
    /// Minor currency units.
    pub amount: i64,
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct InvoiceResponse {
    pub id: String,
    pub tenant_id: String,
    pub order_id: String,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub created_at: String,
}

impl From<Invoice> for InvoiceResponse {
    fn from(i: Invoice) -> Self {
        Self {
            id: i.id.to_string(),
            tenant_id: i.tenant_id.0,
            order_id: i.order_id.to_string(),
            amount: i.amount.0,
            currency: i.currency,
            status: i.status.as_str().to_string(),
            created_at: i.created_at.to_rfc3339(),
        }
    }
}
