//! Invoice handlers (API-GATEWAY.md §4.5). RBAC-guarded, tenant-scoped.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::invoices::{InvoiceResponse, IssueInvoiceRequest};
use crate::api::http::dto::pagination::ListQuery;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `POST /api/v1/erp/invoices` (`erp_invoice_create`).
pub async fn create(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<IssueInvoiceRequest>,
) -> Result<(StatusCode, Json<InvoiceResponse>), ApiError> {
    auth.0.require_permission("erp_invoice_create")?;
    let tenant = TenantId(auth.0.tenant_id);
    let invoice = state
        .invoices
        .issue_invoice(&tenant, body.order_id, body.amount, body.currency)
        .await?;
    Ok((StatusCode::CREATED, Json(invoice.into())))
}

/// `GET /api/v1/erp/invoices` (`erp_invoice_read`).
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<InvoiceResponse>>, ApiError> {
    auth.0.require_permission("erp_invoice_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let invoices = state
        .invoices
        .list(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(InvoiceResponse::from)
        .collect();
    Ok(Json(invoices))
}

/// `GET /api/v1/erp/invoices/{invoice_id}` (`erp_invoice_read`).
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(invoice_id): Path<Uuid>,
) -> Result<Json<InvoiceResponse>, ApiError> {
    auth.0.require_permission("erp_invoice_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let invoice = state.invoices.get(&tenant, &invoice_id).await?;
    Ok(Json(invoice.into()))
}

/// `POST /api/v1/erp/invoices/{invoice_id}/cancel` (`erp_invoice_cancel`).
pub async fn cancel(
    State(state): State<AppState>,
    auth: Auth,
    Path(invoice_id): Path<Uuid>,
) -> Result<Json<InvoiceResponse>, ApiError> {
    auth.0.require_permission("erp_invoice_cancel")?;
    let tenant = TenantId(auth.0.tenant_id);
    let invoice = state.invoices.cancel(&tenant, &invoice_id).await?;
    Ok(Json(invoice.into()))
}
