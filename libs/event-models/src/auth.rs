//! Auth & tenant events published by `auth-svc` (`EVENTS.md` §3.1).

use serde::{Deserialize, Serialize};

use crate::EventHeader;

/// NATS subjects for auth events (`platform.<domain>.<entity>.<action>`).
pub mod subjects {
    pub const USER_REGISTERED: &str = "platform.auth.user.registered";
    pub const USER_UPDATED: &str = "platform.auth.user.updated";
    pub const TENANT_CREATED: &str = "platform.auth.tenant.created";
}

/// A new user account was created (`EVENTS.md` §3.1.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRegistered {
    pub header: EventHeader,
    pub user_id: String,
    pub email: String,
    pub display_name: String,
    pub is_active: bool,
}

/// An existing user account was updated (`EVENTS.md` §3.1.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUpdated {
    pub header: EventHeader,
    pub user_id: String,
    pub email: String,
    pub display_name: String,
    pub is_active: bool,
}

/// A new tenant was provisioned (`EVENTS.md` §3.1.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantCreated {
    pub header: EventHeader,
    pub tenant_id: String,
    pub tenant_name: String,
    pub plan: String,
}

/// In-process convenience wrapper so domain code can hand a single type to the
/// `EventPublisher` port. On the wire, only the inner payload struct is serialized,
/// published on [`AuthEvent::subject`].
#[derive(Debug, Clone)]
pub enum AuthEvent {
    UserRegistered(UserRegistered),
    UserUpdated(UserUpdated),
    TenantCreated(TenantCreated),
}

impl AuthEvent {
    pub fn subject(&self) -> &'static str {
        match self {
            AuthEvent::UserRegistered(_) => subjects::USER_REGISTERED,
            AuthEvent::UserUpdated(_) => subjects::USER_UPDATED,
            AuthEvent::TenantCreated(_) => subjects::TENANT_CREATED,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            AuthEvent::UserRegistered(_) => "UserRegistered",
            AuthEvent::UserUpdated(_) => "UserUpdated",
            AuthEvent::TenantCreated(_) => "TenantCreated",
        }
    }

    pub fn header(&self) -> &EventHeader {
        match self {
            AuthEvent::UserRegistered(e) => &e.header,
            AuthEvent::UserUpdated(e) => &e.header,
            AuthEvent::TenantCreated(e) => &e.header,
        }
    }

    /// Serialize the inner payload (the wire format) to JSON bytes.
    pub fn payload_json(&self) -> serde_json::Result<Vec<u8>> {
        match self {
            AuthEvent::UserRegistered(e) => serde_json::to_vec(e),
            AuthEvent::UserUpdated(e) => serde_json::to_vec(e),
            AuthEvent::TenantCreated(e) => serde_json::to_vec(e),
        }
    }
}
