//! Overview handler (`/api/v1/reporting/overview`). Headline KPIs across the
//! read models reporting currently projects (revenue + orders).

use axum::extract::State;
use axum::Json;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::report::ReportEnvelope;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `GET /api/v1/reporting/overview` (`reporting_dashboard_view`).
pub async fn overview(
    State(state): State<AppState>,
    auth: Auth,
) -> Result<Json<ReportEnvelope>, ApiError> {
    auth.0.require_permission("reporting_dashboard_view")?;
    let tenant = TenantId(auth.0.tenant_id);
    let sales = state.sales.get_summary(&tenant).await?;
    let orders = state.orders.overview(&tenant).await?;
    Ok(Json(ReportEnvelope::new(
        "overview",
        serde_json::json!({
            "revenue": {
                "total_paid": sales.total_paid.0,
                "payment_count": sales.payment_count,
            },
            "orders": {
                "count": orders.order_count,
                "total_amount": orders.total_amount.0,
            },
        }),
    )))
}
