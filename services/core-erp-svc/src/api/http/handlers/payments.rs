//! Order payment handlers (API-GATEWAY.md §4.2). RBAC-guarded, tenant-scoped.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::payments::{PaymentResponse, RecordPaymentRequest};
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `POST /api/v1/erp/orders/{order_id}/payments` (`erp_order_update`).
pub async fn record(
    State(state): State<AppState>,
    auth: Auth,
    Path(order_id): Path<Uuid>,
    Json(body): Json<RecordPaymentRequest>,
) -> Result<(StatusCode, Json<PaymentResponse>), ApiError> {
    auth.0.require_permission("erp_order_update")?;
    let tenant = TenantId(auth.0.tenant_id);
    let payment = state
        .payments
        .record_payment(&tenant, &order_id, body.amount, body.payment_method)
        .await?;
    Ok((StatusCode::CREATED, Json(payment.into())))
}

/// `GET /api/v1/erp/orders/{order_id}/payments` (`erp_order_read`).
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Path(order_id): Path<Uuid>,
) -> Result<Json<Vec<PaymentResponse>>, ApiError> {
    auth.0.require_permission("erp_order_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let payments = state
        .payments
        .list_payments(&tenant, &order_id)
        .await?
        .into_iter()
        .map(PaymentResponse::from)
        .collect();
    Ok(Json(payments))
}
