//! Activity handlers (ARCHITECTURE.md §3.3). RBAC-guarded, tenant-scoped.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::api::http::dto::activities::{
    ActivityResponse, LogActivityRequest, UpdateActivityRequest,
};
use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::pagination::ListQuery;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `POST /api/v1/crm/activities` (`crm_activity_create`).
pub async fn create(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<LogActivityRequest>,
) -> Result<(StatusCode, Json<ActivityResponse>), ApiError> {
    auth.0.require_permission("crm_activity_create")?;
    let tenant = TenantId(auth.0.tenant_id);
    let activity = state
        .activities
        .log(
            &tenant,
            body.customer_id,
            body.kind,
            body.subject,
            body.notes,
            body.occurred_at,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(activity.into())))
}

/// `GET /api/v1/crm/activities` (`crm_activity_read`).
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<ActivityResponse>>, ApiError> {
    auth.0.require_permission("crm_activity_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let activities = state
        .activities
        .list(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(ActivityResponse::from)
        .collect();
    Ok(Json(activities))
}

/// `GET /api/v1/crm/activities/{activity_id}` (`crm_activity_read`).
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(activity_id): Path<Uuid>,
) -> Result<Json<ActivityResponse>, ApiError> {
    auth.0.require_permission("crm_activity_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let activity = state.activities.get(&tenant, &activity_id).await?;
    Ok(Json(activity.into()))
}

/// `PATCH /api/v1/crm/activities/{activity_id}` (`crm_activity_update`).
pub async fn update(
    State(state): State<AppState>,
    auth: Auth,
    Path(activity_id): Path<Uuid>,
    Json(body): Json<UpdateActivityRequest>,
) -> Result<Json<ActivityResponse>, ApiError> {
    auth.0.require_permission("crm_activity_update")?;
    let tenant = TenantId(auth.0.tenant_id);
    let activity = state
        .activities
        .update(&tenant, &activity_id, body.kind, body.subject, body.notes)
        .await?;
    Ok(Json(activity.into()))
}
