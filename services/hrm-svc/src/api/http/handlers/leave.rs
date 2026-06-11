//! Leave handlers (ARCHITECTURE.md §3.4). RBAC-guarded, tenant-scoped.
//! Approval/rejection are explicit sub-actions guarded by `hrm_leave_approve`.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::leave::{LeaveResponse, RequestLeaveRequest};
use crate::api::http::dto::pagination::ListQuery;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::leave::entities::LeaveStatus;
use crate::domain::shared::types::TenantId;

/// `POST /api/v1/hrm/leave` (`hrm_leave_request`).
pub async fn request(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<RequestLeaveRequest>,
) -> Result<(StatusCode, Json<LeaveResponse>), ApiError> {
    auth.0.require_permission("hrm_leave_request")?;
    let tenant = TenantId(auth.0.tenant_id);
    let leave = state
        .leave
        .request(
            &tenant,
            body.employee_id,
            body.start_date,
            body.end_date,
            body.reason,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(leave.into())))
}

/// `GET /api/v1/hrm/leave` (`hrm_leave_read`).
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<LeaveResponse>>, ApiError> {
    auth.0.require_permission("hrm_leave_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let requests = state
        .leave
        .list(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(LeaveResponse::from)
        .collect();
    Ok(Json(requests))
}

/// `GET /api/v1/hrm/leave/{leave_id}` (`hrm_leave_read`).
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(leave_id): Path<Uuid>,
) -> Result<Json<LeaveResponse>, ApiError> {
    auth.0.require_permission("hrm_leave_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let leave = state.leave.get(&tenant, &leave_id).await?;
    Ok(Json(leave.into()))
}

async fn decide(
    state: AppState,
    auth: Auth,
    leave_id: Uuid,
    new_status: LeaveStatus,
) -> Result<Json<LeaveResponse>, ApiError> {
    auth.0.require_permission("hrm_leave_approve")?;
    let tenant = TenantId(auth.0.tenant_id);
    let leave = state.leave.decide(&tenant, &leave_id, new_status).await?;
    Ok(Json(leave.into()))
}

/// `POST /api/v1/hrm/leave/{leave_id}/approve` (`hrm_leave_approve`).
pub async fn approve(
    State(state): State<AppState>,
    auth: Auth,
    Path(leave_id): Path<Uuid>,
) -> Result<Json<LeaveResponse>, ApiError> {
    decide(state, auth, leave_id, LeaveStatus::Approved).await
}

/// `POST /api/v1/hrm/leave/{leave_id}/reject` (`hrm_leave_approve`).
pub async fn reject(
    State(state): State<AppState>,
    auth: Auth,
    Path(leave_id): Path<Uuid>,
) -> Result<Json<LeaveResponse>, ApiError> {
    decide(state, auth, leave_id, LeaveStatus::Rejected).await
}
