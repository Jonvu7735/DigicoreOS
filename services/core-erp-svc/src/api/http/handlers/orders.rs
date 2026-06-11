//! Order handlers (API-GATEWAY.md §4.1). RBAC-guarded, tenant-scoped. Status
//! transitions are explicit sub-actions (/confirm, /complete, /cancel).

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::orders::{CreateOrderRequest, OrderResponse};
use crate::api::http::dto::pagination::ListQuery;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::orders::entities::OrderStatus;
use crate::domain::shared::types::TenantId;

/// `POST /api/v1/erp/orders` (`erp_order_create`).
pub async fn create(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<OrderResponse>), ApiError> {
    auth.0.require_permission("erp_order_create")?;
    let tenant = TenantId(auth.0.tenant_id);
    let order = state
        .orders
        .create_order(&tenant, body.customer_id, body.total_amount, body.currency)
        .await?;
    Ok((StatusCode::CREATED, Json(order.into())))
}

/// `GET /api/v1/erp/orders` (`erp_order_read`).
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<OrderResponse>>, ApiError> {
    auth.0.require_permission("erp_order_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let orders = state
        .orders
        .list(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(OrderResponse::from)
        .collect();
    Ok(Json(orders))
}

/// `GET /api/v1/erp/orders/{order_id}` (`erp_order_read`).
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(order_id): Path<Uuid>,
) -> Result<Json<OrderResponse>, ApiError> {
    auth.0.require_permission("erp_order_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let order = state.orders.get(&tenant, &order_id).await?;
    Ok(Json(order.into()))
}

async fn transition(
    state: AppState,
    auth: Auth,
    order_id: Uuid,
    permission: &str,
    new_status: OrderStatus,
) -> Result<Json<OrderResponse>, ApiError> {
    auth.0.require_permission(permission)?;
    let tenant = TenantId(auth.0.tenant_id);
    let order = state
        .orders
        .change_status(&tenant, &order_id, new_status)
        .await?;
    Ok(Json(order.into()))
}

/// `POST /api/v1/erp/orders/{order_id}/confirm` (`erp_order_update`).
pub async fn confirm(
    State(state): State<AppState>,
    auth: Auth,
    Path(order_id): Path<Uuid>,
) -> Result<Json<OrderResponse>, ApiError> {
    transition(
        state,
        auth,
        order_id,
        "erp_order_update",
        OrderStatus::Confirmed,
    )
    .await
}

/// `POST /api/v1/erp/orders/{order_id}/complete` (`erp_order_update`).
pub async fn complete(
    State(state): State<AppState>,
    auth: Auth,
    Path(order_id): Path<Uuid>,
) -> Result<Json<OrderResponse>, ApiError> {
    transition(
        state,
        auth,
        order_id,
        "erp_order_update",
        OrderStatus::Completed,
    )
    .await
}

/// `POST /api/v1/erp/orders/{order_id}/cancel` (`erp_order_cancel`).
pub async fn cancel(
    State(state): State<AppState>,
    auth: Auth,
    Path(order_id): Path<Uuid>,
) -> Result<Json<OrderResponse>, ApiError> {
    transition(
        state,
        auth,
        order_id,
        "erp_order_cancel",
        OrderStatus::Cancelled,
    )
    .await
}
