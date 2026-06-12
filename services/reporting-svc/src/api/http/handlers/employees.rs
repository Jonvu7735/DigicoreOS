//! Employee report handler (`/api/v1/reporting/employees`). RBAC-guarded,
//! tenant-scoped.

use axum::extract::{Query, State};
use axum::Json;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::pagination::ListQuery;
use crate::api::http::dto::report::ReportEnvelope;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `GET /api/v1/reporting/employees` (`reporting_dashboard_view`) — detailed
/// employees from the `employee_facts` projection (paginated, newest first).
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<ReportEnvelope>, ApiError> {
    auth.0.require_permission("reporting_dashboard_view")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let total = state.employees.count(&tenant).await?;
    let items: Vec<serde_json::Value> = state
        .employees
        .list(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(|e| {
            serde_json::json!({
                "employee_id": e.employee_id,
                "full_name": e.full_name,
                "position": e.position,
                "created_at": e.created_at.to_rfc3339(),
            })
        })
        .collect();
    Ok(Json(ReportEnvelope::new(
        "employees",
        serde_json::json!({ "total": total, "items": items }),
    )))
}
