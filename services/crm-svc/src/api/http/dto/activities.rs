//! Activity DTOs (`/api/v1/crm/activities`).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::activities::entities::Activity;

#[derive(Debug, Deserialize)]
pub struct LogActivityRequest {
    pub customer_id: Uuid,
    /// `CALL` | `EMAIL` | `MEETING` | `TASK` (case-insensitive).
    pub kind: String,
    pub subject: String,
    #[serde(default)]
    pub notes: Option<String>,
    /// RFC3339 timestamp; defaults to now if omitted.
    #[serde(default)]
    pub occurred_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateActivityRequest {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ActivityResponse {
    pub id: String,
    pub tenant_id: String,
    pub customer_id: String,
    pub kind: String,
    pub subject: String,
    pub notes: Option<String>,
    pub occurred_at: String,
    pub created_at: String,
}

impl From<Activity> for ActivityResponse {
    fn from(a: Activity) -> Self {
        Self {
            id: a.id.to_string(),
            tenant_id: a.tenant_id.0,
            customer_id: a.customer_id.to_string(),
            kind: a.kind.as_str().to_string(),
            subject: a.subject,
            notes: a.notes,
            occurred_at: a.occurred_at.to_rfc3339(),
            created_at: a.created_at.to_rfc3339(),
        }
    }
}
