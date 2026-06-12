//! Inventory summary report handler (`/api/v1/reporting/inventory-summary`).
//! RBAC-guarded, tenant-scoped.

use axum::extract::State;
use axum::Json;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::report::ReportEnvelope;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `GET /api/v1/reporting/inventory-summary` (`reporting_dashboard_view`) —
/// stock-on-hand per (product, warehouse) from the `stock_facts` projection.
pub async fn summary(
    State(state): State<AppState>,
    auth: Auth,
) -> Result<Json<ReportEnvelope>, ApiError> {
    auth.0.require_permission("reporting_dashboard_view")?;
    let tenant = TenantId(auth.0.tenant_id);
    let products: Vec<serde_json::Value> = state
        .inventory
        .summary(&tenant)
        .await?
        .into_iter()
        .map(|l| {
            serde_json::json!({
                "product_id": l.product_id,
                "warehouse_id": l.warehouse_id,
                "quantity": l.quantity,
            })
        })
        .collect();
    Ok(Json(ReportEnvelope::new(
        "inventory-summary",
        serde_json::json!({ "products": products }),
    )))
}
