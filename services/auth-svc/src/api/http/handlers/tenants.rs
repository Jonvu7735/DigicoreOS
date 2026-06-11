//! Tenant-management handlers (API-GATEWAY.md §3.3). Scoped to the caller's own
//! tenant; cross-tenant (super-admin) access is a later concern.

use axum::extract::{Path, State};
use axum::Json;

use crate::api::http::dto::auth::{TenantResponse, UpdateTenantRequest};
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
