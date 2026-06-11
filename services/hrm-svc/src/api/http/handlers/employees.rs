//! Employee handlers (ARCHITECTURE.md §3.4). RBAC-guarded, tenant-scoped.
//! Termination is an explicit sub-action (`/terminate`).

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::api::http::dto::employees::{
    EmployeeResponse, HireEmployeeRequest, TerminateEmployeeRequest,
};
use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::pagination::ListQuery;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `POST /api/v1/hrm/employees` (`hrm_employee_create`).
pub async fn hire(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<HireEmployeeRequest>,
) -> Result<(StatusCode, Json<EmployeeResponse>), ApiError> {
    auth.0.require_permission("hrm_employee_create")?;
    let tenant = TenantId(auth.0.tenant_id);
    let employee = state
        .employees
        .hire(&tenant, body.full_name, body.position, body.email)
        .await?;
    Ok((StatusCode::CREATED, Json(employee.into())))
}

/// `GET /api/v1/hrm/employees` (`hrm_employee_read`).
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<EmployeeResponse>>, ApiError> {
    auth.0.require_permission("hrm_employee_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let employees = state
        .employees
        .list(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(EmployeeResponse::from)
        .collect();
    Ok(Json(employees))
}

/// `GET /api/v1/hrm/employees/{employee_id}` (`hrm_employee_read`).
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(employee_id): Path<Uuid>,
) -> Result<Json<EmployeeResponse>, ApiError> {
    auth.0.require_permission("hrm_employee_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let employee = state.employees.get(&tenant, &employee_id).await?;
    Ok(Json(employee.into()))
}

/// `POST /api/v1/hrm/employees/{employee_id}/terminate` (`hrm_employee_update`).
pub async fn terminate(
    State(state): State<AppState>,
    auth: Auth,
    Path(employee_id): Path<Uuid>,
    Json(body): Json<TerminateEmployeeRequest>,
) -> Result<Json<EmployeeResponse>, ApiError> {
    auth.0.require_permission("hrm_employee_update")?;
    let tenant = TenantId(auth.0.tenant_id);
    let employee = state
        .employees
        .terminate(&tenant, &employee_id, body.reason)
        .await?;
    Ok(Json(employee.into()))
}
