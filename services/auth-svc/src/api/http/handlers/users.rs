//! Admin user-management handlers (API-GATEWAY.md §3.2). Every handler verifies
//! the caller (via `AuthContext`) and enforces the required RBAC permission;
//! all operations are scoped to the caller's tenant.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::api::http::dto::auth::{CreateUserRequest, ListQuery, UpdateUserRequest, UserSummary};
use crate::api::http::dto::error::ApiError;
use crate::api::http::middleware::AuthContext;
use crate::bootstrap::wiring::AppState;
use crate::domain::identity::services::UserView;
use crate::domain::shared::types::{Email, TenantId, UserId};

fn user_summary(tenant_id: &TenantId, view: UserView) -> UserSummary {
    UserSummary {
        id: view.user.id.to_string(),
        email: view.user.email.to_string(),
        display_name: view.user.display_name,
        tenant_id: tenant_id.0.clone(),
        roles: view.roles,
    }
}

/// `POST /api/v1/auth/users` – create a user (`auth_user_create`).
pub async fn create(
    State(state): State<AppState>,
    ctx: AuthContext,
    Json(body): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserSummary>), ApiError> {
    ctx.require_permission("auth_user_create")?;
    let view = state
        .identity
        .create_user(
            &ctx.tenant_id,
            Email(body.email),
            body.password,
            body.display_name,
            body.role,
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(user_summary(&ctx.tenant_id, view)),
    ))
}

/// `GET /api/v1/auth/users` – list users in the tenant (`auth_user_read`).
pub async fn list(
    State(state): State<AppState>,
    ctx: AuthContext,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<UserSummary>>, ApiError> {
    ctx.require_permission("auth_user_read")?;
    let (limit, offset) = query.limit_offset();
    let users = state
        .identity
        .list_users(&ctx.tenant_id, limit, offset)
        .await?
        .into_iter()
        .map(|view| user_summary(&ctx.tenant_id, view))
        .collect();
    Ok(Json(users))
}

/// `GET /api/v1/auth/users/{user_id}` – fetch one user (`auth_user_read`).
pub async fn get(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(user_id): Path<Uuid>,
) -> Result<Json<UserSummary>, ApiError> {
    ctx.require_permission("auth_user_read")?;
    let view = state
        .identity
        .get_user(&ctx.tenant_id, &UserId(user_id))
        .await?;
    Ok(Json(user_summary(&ctx.tenant_id, view)))
}

/// `PATCH /api/v1/auth/users/{user_id}` – partial update (`auth_user_update`).
pub async fn update(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(user_id): Path<Uuid>,
    Json(body): Json<UpdateUserRequest>,
) -> Result<Json<UserSummary>, ApiError> {
    ctx.require_permission("auth_user_update")?;
    let view = state
        .identity
        .update_user(
            &ctx.tenant_id,
            &UserId(user_id),
            body.display_name,
            body.is_active,
        )
        .await?;
    Ok(Json(user_summary(&ctx.tenant_id, view)))
}

/// `DELETE /api/v1/auth/users/{user_id}` – soft-delete by deactivating the user
/// (`auth_user_update`). Returns 204.
pub async fn deactivate(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    ctx.require_permission("auth_user_update")?;
    state
        .identity
        .update_user(&ctx.tenant_id, &UserId(user_id), None, Some(false))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
