//! CRM events published by `crm-svc` (`EVENTS.md` §3.3).

use serde::{Deserialize, Serialize};

use crate::EventHeader;

/// NATS subjects for CRM events (`platform.<domain>.<entity>.<action>`).
pub mod subjects {
    pub const CUSTOMER_CREATED: &str = "platform.crm.customer.created";
    pub const CUSTOMER_UPDATED: &str = "platform.crm.customer.updated";
}

/// A new customer was created (`EVENTS.md` §3.3.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerCreated {
    pub header: EventHeader,
    pub customer_id: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub segment: Option<String>,
}

/// An existing customer was updated (`EVENTS.md` §3.3.2). Carries the full
/// current state (not a delta).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerUpdated {
    pub header: EventHeader,
    pub customer_id: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub segment: Option<String>,
}

/// In-process wrapper so domain code hands a single type to the event publisher.
/// On the wire, only the inner payload struct is serialized, published on
/// [`CrmEvent::subject`].
#[derive(Debug, Clone)]
pub enum CrmEvent {
    CustomerCreated(CustomerCreated),
    CustomerUpdated(CustomerUpdated),
}

impl CrmEvent {
    pub fn subject(&self) -> &'static str {
        match self {
            CrmEvent::CustomerCreated(_) => subjects::CUSTOMER_CREATED,
            CrmEvent::CustomerUpdated(_) => subjects::CUSTOMER_UPDATED,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            CrmEvent::CustomerCreated(_) => "CustomerCreated",
            CrmEvent::CustomerUpdated(_) => "CustomerUpdated",
        }
    }

    pub fn header(&self) -> &EventHeader {
        match self {
            CrmEvent::CustomerCreated(e) => &e.header,
            CrmEvent::CustomerUpdated(e) => &e.header,
        }
    }

    /// Serialize the inner payload (the wire format) to JSON bytes.
    pub fn payload_json(&self) -> serde_json::Result<Vec<u8>> {
        match self {
            CrmEvent::CustomerCreated(e) => serde_json::to_vec(e),
            CrmEvent::CustomerUpdated(e) => serde_json::to_vec(e),
        }
    }
}
