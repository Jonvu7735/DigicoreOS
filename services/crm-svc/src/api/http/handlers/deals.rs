//! Deal handlers (ARCHITECTURE.md §3.3). RBAC-guarded, tenant-scoped. The
//! pipeline move is an explicit sub-action (`/stage`).

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::api::http::dto::deals::{ChangeStageRequest, CreateDealRequest, DealResponse};
use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::pagination::ListQuery;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::deals::entities::DealStage;
use crate::domain::shared::error::DomainError;
use crate::domain::shared::types::TenantId;

/// `POST /api/v1/crm/deals` (`crm_deal_create`).
pub async fn create(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<CreateDealRequest>,
) -> Result<(StatusCode, Json<DealResponse>), ApiError> {
    auth.0.require_permission("crm_deal_create")?;
    let tenant = TenantId(auth.0.tenant_id);
    let deal = state
        .deals
        .create_deal(&tenant, body.customer_id, body.title, body.amount_estimate)
        .await?;
    Ok((StatusCode::CREATED, Json(deal.into())))
}

/// `GET /api/v1/crm/deals` (`crm_deal_read`).
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<DealResponse>>, ApiError> {
    auth.0.require_permission("crm_deal_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let deals = state
        .deals
        .list(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(DealResponse::from)
        .collect();
    Ok(Json(deals))
}

/// `GET /api/v1/crm/deals/{deal_id}` (`crm_deal_read`).
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(deal_id): Path<Uuid>,
) -> Result<Json<DealResponse>, ApiError> {
    auth.0.require_permission("crm_deal_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let deal = state.deals.get(&tenant, &deal_id).await?;
    Ok(Json(deal.into()))
}

/// `POST /api/v1/crm/deals/{deal_id}/stage` (`crm_deal_move_stage`).
pub async fn change_stage(
    State(state): State<AppState>,
    auth: Auth,
    Path(deal_id): Path<Uuid>,
    Json(body): Json<ChangeStageRequest>,
) -> Result<Json<DealResponse>, ApiError> {
    auth.0.require_permission("crm_deal_move_stage")?;
    let tenant = TenantId(auth.0.tenant_id);
    let new_stage = DealStage::parse(&body.stage)
        .ok_or_else(|| DomainError::Validation(format!("unknown stage: {}", body.stage)))?;
    let deal = state
        .deals
        .change_stage(&tenant, &deal_id, new_stage)
        .await?;
    Ok(Json(deal.into()))
}
