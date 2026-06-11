//! Invoice entity (maps to `erp_core_svc.invoices`).

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::shared::types::{Money, TenantId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvoiceStatus {
    Issued,
    Cancelled,
}

impl InvoiceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            InvoiceStatus::Issued => "ISSUED",
            InvoiceStatus::Cancelled => "CANCELLED",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "ISSUED" => Some(InvoiceStatus::Issued),
            "CANCELLED" => Some(InvoiceStatus::Cancelled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Invoice {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub order_id: Uuid,
    pub amount: Money,
    pub currency: String,
    pub status: InvoiceStatus,
    pub created_at: DateTime<Utc>,
}
