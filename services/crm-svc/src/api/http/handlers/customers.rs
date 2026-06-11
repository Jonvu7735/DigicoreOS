//! Customer handlers (ARCHITECTURE.md §3.3). RBAC-guarded, tenant-scoped.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::api::http::dto::customers::{
    CreateCustomerRequest, CustomerResponse, UpdateCustomerRequest,
};
use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::pagination::ListQuery;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `POST /api/v1/crm/customers` (`crm_customer_create`).
pub async fn create(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<CreateCustomerRequest>,
) -> Result<(StatusCode, Json<CustomerResponse>), ApiError> {
    auth.0.require_permission("crm_customer_create")?;
    let tenant = TenantId(auth.0.tenant_id);
    let customer = state
        .customers
        .create(&tenant, body.name, body.email, body.phone, body.segment)
        .await?;
    Ok((StatusCode::CREATED, Json(customer.into())))
}

/// `GET /api/v1/crm/customers` (`crm_customer_read`).
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<CustomerResponse>>, ApiError> {
    auth.0.require_permission("crm_customer_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let customers = state
        .customers
        .list(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(CustomerResponse::from)
        .collect();
    Ok(Json(customers))
}

/// `GET /api/v1/crm/customers/{customer_id}` (`crm_customer_read`).
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(customer_id): Path<Uuid>,
) -> Result<Json<CustomerResponse>, ApiError> {
    auth.0.require_permission("crm_customer_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let customer = state.customers.get(&tenant, &customer_id).await?;
    Ok(Json(customer.into()))
}

/// `PATCH /api/v1/crm/customers/{customer_id}` (`crm_customer_update`).
pub async fn update(
    State(state): State<AppState>,
    auth: Auth,
    Path(customer_id): Path<Uuid>,
    Json(body): Json<UpdateCustomerRequest>,
) -> Result<Json<CustomerResponse>, ApiError> {
    auth.0.require_permission("crm_customer_update")?;
    let tenant = TenantId(auth.0.tenant_id);
    let customer = state
        .customers
        .update(
            &tenant,
            &customer_id,
            body.name,
            body.email,
            body.phone,
            body.segment,
        )
        .await?;
    Ok(Json(customer.into()))
}
