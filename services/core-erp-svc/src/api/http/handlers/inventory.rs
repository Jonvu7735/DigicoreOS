//! Inventory handlers (API-GATEWAY.md §4.3). RBAC-guarded, tenant-scoped.
//! The catalogue has no `erp_inventory_*` permission, so stock operations are
//! guarded by `erp_product_*` (closest fit, SECURITY.md §4.3).

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::inventory::{
    adjustment_response, AdjustmentResponse, CreateAdjustmentRequest, StockLevelResponse,
};
use crate::api::http::dto::pagination::ListQuery;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `GET /api/v1/erp/inventory` (`erp_product_read`).
pub async fn list_stock(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<StockLevelResponse>>, ApiError> {
    auth.0.require_permission("erp_product_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let levels = state
        .inventory
        .list_stock(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(StockLevelResponse::from)
        .collect();
    Ok(Json(levels))
}

/// `POST /api/v1/erp/inventory/adjustments` (`erp_product_update`).
pub async fn create_adjustment(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<CreateAdjustmentRequest>,
) -> Result<(StatusCode, Json<AdjustmentResponse>), ApiError> {
    auth.0.require_permission("erp_product_update")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (adjustment, quantity) = state
        .inventory
        .adjust_stock(
            &tenant,
            body.product_id,
            body.warehouse_id,
            body.delta,
            body.reason,
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(adjustment_response(adjustment, Some(quantity))),
    ))
}

/// `GET /api/v1/erp/inventory/adjustments` (`erp_product_read`).
pub async fn list_adjustments(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<AdjustmentResponse>>, ApiError> {
    auth.0.require_permission("erp_product_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let adjustments = state
        .inventory
        .list_adjustments(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(|a| adjustment_response(a, None))
        .collect();
    Ok(Json(adjustments))
}
