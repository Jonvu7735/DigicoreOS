//! Order DTOs (`/api/v1/erp/orders`).

use serde::{Deserialize, Serialize};

use crate::domain::orders::entities::Order;

#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    pub customer_id: String,
    /// Minor currency units.
    pub total_amount: i64,
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct OrderResponse {
    pub id: String,
    pub tenant_id: String,
    pub customer_id: String,
    pub total_amount: i64,
    pub currency: String,
    pub status: String,
    pub created_at: String,
}

impl From<Order> for OrderResponse {
    fn from(o: Order) -> Self {
        Self {
            id: o.id.to_string(),
            tenant_id: o.tenant_id.0,
            customer_id: o.customer_id,
            total_amount: o.total_amount.0,
            currency: o.currency,
            status: o.status.as_str().to_string(),
            created_at: o.created_at.to_rfc3339(),
        }
    }
}
