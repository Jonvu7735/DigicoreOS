//! Contact handlers (ARCHITECTURE.md §3.3). RBAC-guarded, tenant-scoped.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::api::http::dto::contacts::{
    ContactResponse, CreateContactRequest, UpdateContactRequest,
};
use crate::api::http::dto::error::ApiError;
use crate::api::http::dto::pagination::ListQuery;
use crate::api::http::middleware::Auth;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::TenantId;

/// `POST /api/v1/crm/contacts` (`crm_contact_create`).
pub async fn create(
    State(state): State<AppState>,
    auth: Auth,
    Json(body): Json<CreateContactRequest>,
) -> Result<(StatusCode, Json<ContactResponse>), ApiError> {
    auth.0.require_permission("crm_contact_create")?;
    let tenant = TenantId(auth.0.tenant_id);
    let contact = state
        .contacts
        .create(
            &tenant,
            body.customer_id,
            body.name,
            body.email,
            body.phone,
            body.title,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(contact.into())))
}

/// `GET /api/v1/crm/contacts` (`crm_contact_read`).
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<ContactResponse>>, ApiError> {
    auth.0.require_permission("crm_contact_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let (limit, offset) = query.limit_offset();
    let contacts = state
        .contacts
        .list(&tenant, limit, offset)
        .await?
        .into_iter()
        .map(ContactResponse::from)
        .collect();
    Ok(Json(contacts))
}

/// `GET /api/v1/crm/contacts/{contact_id}` (`crm_contact_read`).
pub async fn get(
    State(state): State<AppState>,
    auth: Auth,
    Path(contact_id): Path<Uuid>,
) -> Result<Json<ContactResponse>, ApiError> {
    auth.0.require_permission("crm_contact_read")?;
    let tenant = TenantId(auth.0.tenant_id);
    let contact = state.contacts.get(&tenant, &contact_id).await?;
    Ok(Json(contact.into()))
}

/// `PATCH /api/v1/crm/contacts/{contact_id}` (`crm_contact_update`).
pub async fn update(
    State(state): State<AppState>,
    auth: Auth,
    Path(contact_id): Path<Uuid>,
    Json(body): Json<UpdateContactRequest>,
) -> Result<Json<ContactResponse>, ApiError> {
    auth.0.require_permission("crm_contact_update")?;
    let tenant = TenantId(auth.0.tenant_id);
    let contact = state
        .contacts
        .update(
            &tenant,
            &contact_id,
            body.name,
            body.email,
            body.phone,
            body.title,
        )
        .await?;
    Ok(Json(contact.into()))
}
