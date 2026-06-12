//! Loyalty handlers (`/api/v1/retail/loyalty`). Role-guarded, tenant-scoped.

use axum::extract::{Path, Query, State};
use axum::Json;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::loyalty::{
    LoyaltyAccountResponse, PointsLedgerEntryResponse, RedeemRequest,
};
use crate::api::http::dto::pagination::{ListQuery, Page};
use crate::api::http::middleware::{Auth, READ_ROLES, WRITE_ROLES};
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `GET /api/v1/retail/loyalty` — list loyalty accounts in the tenant.
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<Page<LoyaltyAccountResponse>>, ApiError> {
    auth.require_any_role(&READ_ROLES)?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let (page, page_size) = query.page_meta();
    let items = state
        .loyalty
        .list(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(LoyaltyAccountResponse::from)
        .collect();
    Ok(Json(Page::new(items, page, page_size)))
}

/// `GET /api/v1/retail/loyalty/{customer_id}` — one customer's account.
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(customer_id): Path<String>,
) -> Result<Json<LoyaltyAccountResponse>, ApiError> {
    auth.require_any_role(&READ_ROLES)?;
    let tenant = TenantId(auth.0.tenant_id);
    let account = state.loyalty.get(&tenant, &customer_id).await?;
    Ok(Json(account.into()))
}

/// `GET /api/v1/retail/loyalty/{customer_id}/ledger` — a customer's points history.
pub async fn ledger(
    State(state): State<AppState>,
    auth: Auth,
    Path(customer_id): Path<String>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<PointsLedgerEntryResponse>>, ApiError> {
    auth.require_any_role(&READ_ROLES)?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let entries = state
        .loyalty
        .list_ledger(&tenant, &customer_id, limit, offset)
        .await?
        .into_iter()
        .map(PointsLedgerEntryResponse::from)
        .collect();
    Ok(Json(entries))
}

/// `POST /api/v1/retail/loyalty/{customer_id}/redeem` — redeem + emit event.
pub async fn redeem(
    State(state): State<AppState>,
    auth: Auth,
    Path(customer_id): Path<String>,
    Json(body): Json<RedeemRequest>,
) -> Result<Json<LoyaltyAccountResponse>, ApiError> {
    auth.require_any_role(&WRITE_ROLES)?;
    let tenant = TenantId(auth.0.tenant_id);
    let account = state
        .loyalty
        .redeem(&tenant, &customer_id, body.points)
        .await?;
    Ok(Json(account.into()))
}
