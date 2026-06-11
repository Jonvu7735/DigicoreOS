//! Attendance handlers (ARCHITECTURE.md §3.4). RBAC-guarded, tenant-scoped.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::api::http::dto::attendance::{AttendanceResponse, RecordAttendanceRequest};
use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::pagination::ListQuery;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `POST /api/v1/hrm/attendance` (`hrm_attendance_create`).
pub async fn record(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<RecordAttendanceRequest>,
) -> Result<(StatusCode, Json<AttendanceResponse>), ApiError> {
    auth.0.require_permission("hrm_attendance_create")?;
    let tenant = TenantId(auth.0.tenant_id);
    let record = state
        .attendance
        .record(
            &tenant,
            body.employee_id,
            body.date,
            body.check_in,
            body.check_out,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(record.into())))
}

/// `GET /api/v1/hrm/attendance` (`hrm_attendance_read`).
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<AttendanceResponse>>, ApiError> {
    auth.0.require_permission("hrm_attendance_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let records = state
        .attendance
        .list(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(AttendanceResponse::from)
        .collect();
    Ok(Json(records))
}

/// `GET /api/v1/hrm/attendance/{attendance_id}` (`hrm_attendance_read`).
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(attendance_id): Path<Uuid>,
) -> Result<Json<AttendanceResponse>, ApiError> {
    auth.0.require_permission("hrm_attendance_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let record = state.attendance.get(&tenant, &attendance_id).await?;
    Ok(Json(record.into()))
}
