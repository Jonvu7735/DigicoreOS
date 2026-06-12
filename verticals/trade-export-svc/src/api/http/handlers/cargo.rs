//! Cargo-line handlers (`/api/v1/trade-export/shipments/{shipment_id}/cargo`).
//! Role-guarded, tenant-scoped — same policy as the shipment handlers.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::api::http::dto::cargo::{AddCargoLineRequest, CargoLineResponse};
use crate::api::http::dto::error::ApiError;
use crate::api::http::middleware::{Auth, READ_ROLES, WRITE_ROLES};
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `POST /shipments/{shipment_id}/cargo` — add a cargo line to a shipment.
pub async fn add(
    State(state): State<AppState>,
    auth: Auth,
    Path(shipment_id): Path<Uuid>,
    Json(body): Json<AddCargoLineRequest>,
) -> Result<(StatusCode, Json<CargoLineResponse>), ApiError> {
    auth.require_any_role(&WRITE_ROLES)?;
    let tenant = TenantId(auth.0.tenant_id);
    let line = state
        .shipments
        .add_cargo_line(&tenant, &shipment_id, body.into())
        .await?;
    Ok((StatusCode::CREATED, Json(line.into())))
}

/// `GET /shipments/{shipment_id}/cargo` — list a shipment's cargo lines.
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Path(shipment_id): Path<Uuid>,
) -> Result<Json<Vec<CargoLineResponse>>, ApiError> {
    auth.require_any_role(&READ_ROLES)?;
    let tenant = TenantId(auth.0.tenant_id);
    let lines = state
        .shipments
        .list_cargo_lines(&tenant, &shipment_id)
        .await?
        .into_iter()
        .map(CargoLineResponse::from)
        .collect();
    Ok(Json(lines))
}
