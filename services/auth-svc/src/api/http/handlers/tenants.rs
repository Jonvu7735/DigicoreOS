//! Tenant-management handlers (API-GATEWAY.md §3.3). Scoped to the caller's own
//! tenant; cross-tenant (super-admin) access is a later concern.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;

use crate::api::http::dto::auth::{
    CreateTenantRequest, ListQuery, TenantResponse, UpdateTenantRequest,
};
use crate::api::http::dto::error::ApiError;
use crate::api::http::middleware::AuthContext;
use crate::bootstrap::wiring::AppState;
use crate::domain::identity::entities::Tenant;
use crate::domain::shared::error::DomainError;
use crate::domain::shared::types::TenantId;

fn tenant_response(tenant: Tenant) -> TenantResponse {
    TenantResponse {
        id: tenant.id.0,
        name: tenant.name,
        plan: tenant.plan,
        is_active: tenant.is_active,
        created_at: tenant.created_at.to_rfc3339(),
    }
}

/// Reject access to any tenant other than the caller's (no cross-tenant reads).
/// Returns 404 rather than 403 so foreign tenant ids are not confirmed.
fn ensure_own_tenant(ctx: &AuthContext, path_tenant: &str) -> Result<(), ApiError> {
    if ctx.tenant_id.0 == path_tenant {
        Ok(())
    } else {
        Err(DomainError::NotFound(format!("tenant {path_tenant}")).into())
    }
}

/// `GET /api/v1/auth/tenants` (`auth_tenant_manage`) — list all tenants. This is
/// a platform super-admin operation (cross-tenant), so it is gated on
/// `auth_tenant_manage`, which only the SUPER_ADMIN role holds.
pub async fn list(
    State(state): State<AppState>,
    ctx: AuthContext,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<TenantResponse>>, ApiError> {
    ctx.require_permission("auth_tenant_manage")?;
    let (limit, offset) = query.limit_offset();
    let tenants = state
        .identity
        .list_tenants(limit, offset)
        .await?
        .into_iter()
        .map(tenant_response)
        .collect();
    Ok(Json(tenants))
}

/// `POST /api/v1/auth/tenants` (`auth_tenant_manage`) — create a tenant
/// (platform super-admin). Default roles are seeded so the tenant is usable.
pub async fn create(
    State(state): State<AppState>,
    ctx: AuthContext,
    Json(body): Json<CreateTenantRequest>,
) -> Result<(StatusCode, Json<TenantResponse>), ApiError> {
    ctx.require_permission("auth_tenant_manage")?;
    let tenant = state.identity.create_tenant(body.name, body.plan).await?;
    Ok((StatusCode::CREATED, Json(tenant_response(tenant))))
}

/// `GET /api/v1/auth/tenants/{tenant_id}` (`auth_tenant_read`).
pub async fn get(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(tenant_id): Path<String>,
) -> Result<Json<TenantResponse>, ApiError> {
    ctx.require_permission("auth_tenant_read")?;
    ensure_own_tenant(&ctx, &tenant_id)?;
    let tenant = state.identity.get_tenant(&TenantId(tenant_id)).await?;
    Ok(Json(tenant_response(tenant)))
}

/// `PATCH /api/v1/auth/tenants/{tenant_id}` (`auth_tenant_update_plan`).
pub async fn update(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(tenant_id): Path<String>,
    Json(body): Json<UpdateTenantRequest>,
) -> Result<Json<TenantResponse>, ApiError> {
    ctx.require_permission("auth_tenant_update_plan")?;
    ensure_own_tenant(&ctx, &tenant_id)?;
    let tenant = state
        .identity
        .update_tenant(&TenantId(tenant_id), body.name, body.plan, body.is_active)
        .await?;
    Ok(Json(tenant_response(tenant)))
}
