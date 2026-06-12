//! CRM funnel report handler (`/api/v1/reporting/crm-funnel`). RBAC-guarded,
//! tenant-scoped.

use axum::extract::State;
use axum::Json;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::report::ReportEnvelope;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `GET /api/v1/reporting/crm-funnel` (`reporting_dashboard_view`) — deal count
/// per current pipeline stage, from the `deal_facts` projection.
pub async fn funnel(
    State(state): State<AppState>,
    auth: Auth,
) -> Result<Json<ReportEnvelope>, ApiError> {
    auth.0.require_permission("reporting_dashboard_view")?;
    let tenant = TenantId(auth.0.tenant_id);
    let stages: Vec<serde_json::Value> = state
        .deals
        .funnel(&tenant)
        .await?
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "stage": s.stage,
                "deal_count": s.deal_count,
            })
        })
        .collect();
    Ok(Json(ReportEnvelope::new(
        "crm-funnel",
        serde_json::json!({ "stages": stages }),
    )))
}
