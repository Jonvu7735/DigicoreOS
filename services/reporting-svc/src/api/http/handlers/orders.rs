//! Order report handler (`/api/v1/reporting/orders`). RBAC-guarded, tenant-scoped.

use axum::extract::{Query, State};
use axum::Json;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::pagination::ListQuery;
use crate::api::http::dto::report::ReportEnvelope;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `GET /api/v1/reporting/orders` (`reporting_dashboard_view`) — detailed orders
/// from the `order_facts` projection (paginated, newest first).
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<ReportEnvelope>, ApiError> {
    auth.0.require_permission("reporting_dashboard_view")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let items: Vec<serde_json::Value> = state
        .orders
        .list(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(|o| {
            serde_json::json!({
                "order_id": o.order_id,
                "customer_id": o.customer_id,
                "total_amount": o.total_amount.0,
                "currency": o.currency,
                "status": o.status,
                "created_at": o.created_at.to_rfc3339(),
            })
        })
        .collect();
    Ok(Json(ReportEnvelope::new(
        "orders",
        serde_json::json!({ "items": items }),
    )))
}
