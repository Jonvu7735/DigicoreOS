//! ERP events published by `core-erp-svc` (`EVENTS.md` §3.2).

use serde::{Deserialize, Serialize};

use crate::EventHeader;

/// NATS subjects for ERP events (`platform.<domain>.<entity>.<action>`).
pub mod subjects {
    pub const ORDER_CREATED: &str = "platform.erp.order.created";
    pub const ORDER_STATUS_CHANGED: &str = "platform.erp.order.status_changed";
    pub const ORDER_PAID: &str = "platform.erp.order.paid";
    pub const STOCK_ADJUSTED: &str = "platform.erp.inventory.stock_adjusted";
    pub const INVOICE_ISSUED: &str = "platform.erp.invoice.issued";
}

/// A new order was created (`EVENTS.md` §3.2.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCreated {
    pub header: EventHeader,
    pub order_id: String,
    pub customer_id: String,
    /// Minor currency units (e.g. cents) to avoid floating point.
    pub total_amount: i64,
    pub currency: String,
    /// e.g. `NEW`.
    pub status: String,
}

/// An order moved between statuses (`EVENTS.md` §3.2.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderStatusChanged {
    pub header: EventHeader,
    pub order_id: String,
    pub old_status: String,
    pub new_status: String,
}

/// An order was paid (`EVENTS.md` §3.2.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPaid {
    pub header: EventHeader,
    pub order_id: String,
    pub amount_paid: i64,
    pub payment_method: String,
}

/// Inventory was adjusted (`EVENTS.md` §3.2.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockAdjusted {
    pub header: EventHeader,
    pub product_id: String,
    pub warehouse_id: String,
    /// Signed quantity delta (positive = increase).
    pub delta: i64,
    /// e.g. `order`, `manual_adjustment`.
    pub reason: String,
}

/// An invoice was issued (`EVENTS.md` §3.2.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceIssued {
    pub header: EventHeader,
    pub invoice_id: String,
    pub order_id: String,
    pub amount: i64,
    pub currency: String,
}

/// In-process wrapper so domain code hands a single type to the event publisher.
/// On the wire, only the inner payload struct is serialized, published on
/// [`ErpEvent::subject`].
#[derive(Debug, Clone)]
pub enum ErpEvent {
    OrderCreated(OrderCreated),
    OrderStatusChanged(OrderStatusChanged),
    OrderPaid(OrderPaid),
    StockAdjusted(StockAdjusted),
    InvoiceIssued(InvoiceIssued),
}

impl ErpEvent {
    pub fn subject(&self) -> &'static str {
        match self {
            ErpEvent::OrderCreated(_) => subjects::ORDER_CREATED,
            ErpEvent::OrderStatusChanged(_) => subjects::ORDER_STATUS_CHANGED,
            ErpEvent::OrderPaid(_) => subjects::ORDER_PAID,
            ErpEvent::StockAdjusted(_) => subjects::STOCK_ADJUSTED,
            ErpEvent::InvoiceIssued(_) => subjects::INVOICE_ISSUED,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            ErpEvent::OrderCreated(_) => "OrderCreated",
            ErpEvent::OrderStatusChanged(_) => "OrderStatusChanged",
            ErpEvent::OrderPaid(_) => "OrderPaid",
            ErpEvent::StockAdjusted(_) => "StockAdjusted",
            ErpEvent::InvoiceIssued(_) => "InvoiceIssued",
        }
    }

    pub fn header(&self) -> &EventHeader {
        match self {
            ErpEvent::OrderCreated(e) => &e.header,
            ErpEvent::OrderStatusChanged(e) => &e.header,
            ErpEvent::OrderPaid(e) => &e.header,
            ErpEvent::StockAdjusted(e) => &e.header,
            ErpEvent::InvoiceIssued(e) => &e.header,
        }
    }

    /// Serialize the inner payload (the wire format) to JSON bytes.
    pub fn payload_json(&self) -> serde_json::Result<Vec<u8>> {
        match self {
            ErpEvent::OrderCreated(e) => serde_json::to_vec(e),
            ErpEvent::OrderStatusChanged(e) => serde_json::to_vec(e),
            ErpEvent::OrderPaid(e) => serde_json::to_vec(e),
            ErpEvent::StockAdjusted(e) => serde_json::to_vec(e),
            ErpEvent::InvoiceIssued(e) => serde_json::to_vec(e),
        }
    }
}
