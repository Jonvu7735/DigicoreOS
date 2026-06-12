//! Customer report handler (`/api/v1/reporting/customers`). RBAC-guarded,
//! tenant-scoped.

use axum::extract::{Query, State};
use axum::Json;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::pagination::ListQuery;
use crate::api::http::dto::report::ReportEnvelope;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `GET /api/v1/reporting/customers` (`reporting_dashboard_view`) — detailed
/// customers from the `customer_facts` projection (paginated, newest first).
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<ReportEnvelope>, ApiError> {
    auth.0.require_permission("reporting_dashboard_view")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let total = state.customers.count(&tenant).await?;
    let items: Vec<serde_json::Value> = state
        .customers
        .list(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(|c| {
            serde_json::json!({
                "customer_id": c.customer_id,
                "name": c.name,
                "email": c.email,
                "segment": c.segment,
                "created_at": c.created_at.to_rfc3339(),
            })
        })
        .collect();
    Ok(Json(ReportEnvelope::new(
        "customers",
        serde_json::json!({ "total": total, "items": items }),
    )))
}
