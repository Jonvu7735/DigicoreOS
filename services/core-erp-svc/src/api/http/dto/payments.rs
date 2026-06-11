//! Payment DTOs (`/api/v1/erp/orders/{id}/payments`).

use serde::{Deserialize, Serialize};

use crate::domain::payments::entities::Payment;

#[derive(Debug, Deserialize)]
pub struct RecordPaymentRequest {
    /// Minor currency units.
    pub amount: i64,
    pub payment_method: String,
}

#[derive(Debug, Serialize)]
pub struct PaymentResponse {
    pub id: String,
    pub tenant_id: String,
    pub order_id: String,
    pub amount: i64,
    pub method: String,
    pub created_at: String,
}

impl From<Payment> for PaymentResponse {
    fn from(p: Payment) -> Self {
        Self {
            id: p.id.to_string(),
            tenant_id: p.tenant_id.0,
            order_id: p.order_id.to_string(),
            amount: p.amount.0,
            method: p.method,
            created_at: p.created_at.to_rfc3339(),
        }
    }
}
