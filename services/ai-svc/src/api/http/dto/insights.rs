//! Insight DTOs (`/api/v1/ai/insight`, `/api/v1/ai/insights`).

use serde::{Deserialize, Serialize};

use crate::domain::insights::entities::Insight;

#[derive(Debug, Deserialize)]
pub struct GenerateInsightRequest {
    /// Optional nudge for the classification (e.g. `churn_risk`).
    #[serde(default)]
    pub category_hint: Option<String>,
    /// Free-form context to analyse.
    #[serde(default)]
    pub context: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct InsightResponse {
    pub id: String,
    pub tenant_id: String,
    pub category: String,
    pub summary: String,
    pub source_ref: Option<String>,
    pub created_at: String,
}

impl From<Insight> for InsightResponse {
    fn from(i: Insight) -> Self {
        Self {
            id: i.id.to_string(),
            tenant_id: i.tenant_id.0,
            category: i.category,
            summary: i.summary,
            source_ref: i.source_ref,
            created_at: i.created_at.to_rfc3339(),
        }
    }
}
