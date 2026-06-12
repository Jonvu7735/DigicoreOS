//! HRM summary report handler (`/api/v1/reporting/hrm-summary`). RBAC-guarded,
//! tenant-scoped. Combines headcount (employees projection) with attendance.

use axum::extract::State;
use axum::Json;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::report::ReportEnvelope;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `GET /api/v1/reporting/hrm-summary` (`reporting_dashboard_view`) — headcount
/// (from `employee_facts`) plus attendance rollup (from `attendance_facts`).
pub async fn summary(
    State(state): State<AppState>,
    auth: Auth,
) -> Result<Json<ReportEnvelope>, ApiError> {
    auth.0.require_permission("reporting_dashboard_view")?;
    let tenant = TenantId(auth.0.tenant_id);
    let headcount = state.employees.count(&tenant).await?;
    let attendance = state.attendance.summary(&tenant).await?;
    Ok(Json(ReportEnvelope::new(
        "hrm-summary",
        serde_json::json!({
            "headcount": headcount,
            "attendance": {
                "records": attendance.record_count,
                "present_employees": attendance.present_employees,
            },
        }),
    )))
}
