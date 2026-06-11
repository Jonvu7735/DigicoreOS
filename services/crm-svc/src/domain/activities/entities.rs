//! Activity entity + kind (maps to `crm_svc.activities`). A logged interaction
//! with a customer.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::shared::types::TenantId;

/// The kind of logged interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityKind {
    Call,
    Email,
    Meeting,
    Task,
}

impl ActivityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ActivityKind::Call => "CALL",
            ActivityKind::Email => "EMAIL",
            ActivityKind::Meeting => "MEETING",
            ActivityKind::Task => "TASK",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "CALL" => Some(ActivityKind::Call),
            "EMAIL" => Some(ActivityKind::Email),
            "MEETING" => Some(ActivityKind::Meeting),
            "TASK" => Some(ActivityKind::Task),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Activity {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub customer_id: Uuid,
    pub kind: ActivityKind,
    pub subject: String,
    pub notes: Option<String>,
    /// When the interaction happened (may be backdated; defaults to now).
    pub occurred_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
