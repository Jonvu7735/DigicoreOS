//! Sales dashboard handler (ARCHITECTURE.md §3.5). RBAC-guarded, tenant-scoped.

use axum::extract::State;
use axum::Json;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::sales::SalesSummaryResponse;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `GET /api/v1/reporting/sales-summary` (`reporting_dashboard_view`).
pub async fn summary(
    State(state): State<AppState>,
    auth: Auth,
) -> Result<Json<SalesSummaryResponse>, ApiError> {
    auth.0.require_permission("reporting_dashboard_view")?;
    let tenant = TenantId(auth.0.tenant_id);
    let summary = state.sales.get_summary(&tenant).await?;
    Ok(Json(summary.into()))
}
