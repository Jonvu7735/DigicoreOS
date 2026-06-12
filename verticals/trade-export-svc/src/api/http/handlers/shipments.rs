//! Shipment handlers (`/api/v1/trade-export/shipments`). Role-guarded, tenant-scoped.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::pagination::ListQuery;
use crate::api::http::dto::shipments::{CreateShipmentRequest, ShipmentResponse};
use crate::api::http::middleware::{Auth, READ_ROLES, WRITE_ROLES};
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `POST /api/v1/trade-export/shipments` — create an export shipment.
pub async fn create(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<CreateShipmentRequest>,
) -> Result<(StatusCode, Json<ShipmentResponse>), ApiError> {
    auth.require_any_role(&WRITE_ROLES)?;
    let tenant = TenantId(auth.0.tenant_id);
    let shipment = state
        .shipments
        .create(
            &tenant,
            body.destination_country,
            body.incoterm,
            body.order_id,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(shipment.into())))
}

/// `GET /api/v1/trade-export/shipments` — list shipments in the tenant.
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<ShipmentResponse>>, ApiError> {
    auth.require_any_role(&READ_ROLES)?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let shipments = state
        .shipments
        .list(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(ShipmentResponse::from)
        .collect();
    Ok(Json(shipments))
}

/// `GET /api/v1/trade-export/shipments/{shipment_id}` — shipment detail.
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(shipment_id): Path<Uuid>,
) -> Result<Json<ShipmentResponse>, ApiError> {
    auth.require_any_role(&READ_ROLES)?;
    let tenant = TenantId(auth.0.tenant_id);
    let shipment = state.shipments.get(&tenant, &shipment_id).await?;
    Ok(Json(shipment.into()))
}

/// `POST /api/v1/trade-export/shipments/{shipment_id}/book` — book + emit event.
pub async fn book(
    State(state): State<AppState>,
    auth: Auth,
    Path(shipment_id): Path<Uuid>,
) -> Result<Json<ShipmentResponse>, ApiError> {
    auth.require_any_role(&WRITE_ROLES)?;
    let tenant = TenantId(auth.0.tenant_id);
    let shipment = state.shipments.book(&tenant, &shipment_id).await?;
    Ok(Json(shipment.into()))
}
