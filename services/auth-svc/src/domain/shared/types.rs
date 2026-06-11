//! Strongly-typed domain primitives (`UserId`, `TenantId`, ...) and the
//! `Clock` port. Newtypes prevent accidentally mixing up raw strings/uuids.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Tenant identifier. Kept as TEXT to match event payloads & JWT claims
/// (`EVENTS.md` header `tenant_id`, AUTH-FLOW.md claim `tenant_id`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TenantId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoleId(pub Uuid);

/// Permission code, e.g. `erp_order_create`, `crm_customer_read`
/// (SECURITY.md RBAC matrix).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PermissionCode(pub String);

/// Normalized email address.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Email(pub String);

impl std::fmt::Display for TenantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Time source port. Domain code never calls `Utc::now()` directly – this keeps
/// services deterministic in tests. Implemented by `infra/time/clock.rs`.
pub trait Clock: Send + Sync {
    fn now_utc(&self) -> DateTime<Utc>;
}
