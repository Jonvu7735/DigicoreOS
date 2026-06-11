//! Order entity + status machine (maps to `erp_core_svc.orders`).

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::shared::types::{Money, TenantId};

/// Order lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    New,
    Confirmed,
    Completed,
    Cancelled,
}

impl OrderStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            OrderStatus::New => "NEW",
            OrderStatus::Confirmed => "CONFIRMED",
            OrderStatus::Completed => "COMPLETED",
            OrderStatus::Cancelled => "CANCELLED",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "NEW" => Some(OrderStatus::New),
            "CONFIRMED" => Some(OrderStatus::Confirmed),
            "COMPLETED" => Some(OrderStatus::Completed),
            "CANCELLED" => Some(OrderStatus::Cancelled),
            _ => None,
        }
    }

    /// Allowed transitions: NEW→CONFIRMED→COMPLETED; NEW/CONFIRMED→CANCELLED.
    pub fn can_transition_to(self, next: OrderStatus) -> bool {
        use OrderStatus::*;
        matches!(
            (self, next),
            (New, Confirmed) | (New, Cancelled) | (Confirmed, Completed) | (Confirmed, Cancelled)
        )
    }
}

#[derive(Debug, Clone)]
pub struct Order {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub customer_id: String,
    pub total_amount: Money,
    pub currency: String,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
}
