//! Snapshot handlers (ARCHITECTURE.md §3.5). RBAC-guarded, tenant-scoped.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::pagination::ListQuery;
use crate::api::http::dto::snapshots::{CreateSnapshotRequest, SnapshotResponse};
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `POST /api/v1/reporting/snapshots` (`reporting_report_export`).
pub async fn create(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<CreateSnapshotRequest>,
) -> Result<(StatusCode, Json<SnapshotResponse>), ApiError> {
    auth.0.require_permission("reporting_report_export")?;
    let tenant = TenantId(auth.0.tenant_id);
    let snapshot = state
        .snapshots
        .create_snapshot(&tenant, body.snapshot_type)
        .await?;
    Ok((StatusCode::CREATED, Json(snapshot.into())))
}

/// `GET /api/v1/reporting/snapshots` (`reporting_dashboard_view`).
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<SnapshotResponse>>, ApiError> {
    auth.0.require_permission("reporting_dashboard_view")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let snapshots = state
        .snapshots
        .list(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(SnapshotResponse::from)
        .collect();
    Ok(Json(snapshots))
}
