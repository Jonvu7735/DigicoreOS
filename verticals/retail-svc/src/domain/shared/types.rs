//! Strongly-typed domain primitives + the `Clock` port.

use chrono::{DateTime, Utc};

/// Tenant identifier (TEXT, matches JWT `tenant_id` / event headers).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TenantId(pub String);

impl std::fmt::Display for TenantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Time source port (deterministic in tests). Implemented by infra/time.
pub trait Clock: Send + Sync {
    fn now_utc(&self) -> DateTime<Utc>;
}
