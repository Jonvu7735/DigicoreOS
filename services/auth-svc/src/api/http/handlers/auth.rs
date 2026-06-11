//! Authentication handlers (AUTH-FLOW.md §4–6).
//!
//! Pattern for every handler: deserialize DTO -> call `IdentityService` ->
//! map `LoginOutcome`/`DomainError` to response DTO / `ApiError`.
//! The domain methods currently return `Internal("not implemented")`, so these
//! endpoints respond 500 with a structured body until Phase 1.2 lands.

use axum::extract::State;
use axum::Json;

use crate::api::http::dto::auth::{
    LoginRequest, LoginResponse, LogoutRequest, RefreshRequest, RefreshResponse, UserSummary,
};
use crate::api::http::dto::error::ApiError;
use crate::bootstrap::wiring::AppState;
use crate::domain::shared::types::{Email, TenantId};

/// `POST /api/v1/auth/login`
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let outcome = state
        .identity
        .login(
            Email(body.email),
            body.password,
            body.tenant_id.map(TenantId),
        )
        .await?;

    Ok(Json(LoginResponse {
        access_token: outcome.access.token,
        token_type: "Bearer".into(),
        expires_in: outcome.access.expires_in,
        refresh_token: outcome.refresh_token,
        user: UserSummary {
            id: outcome.user.id.to_string(),
            email: outcome.user.email.to_string(),
            display_name: outcome.user.display_name,
            tenant_id: outcome.tenant_id.0,
            roles: outcome.roles,
        },
    }))
}

/// `POST /api/v1/auth/refresh`
pub async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, ApiError> {
    let outcome = state.identity.refresh(body.refresh_token).await?;

    Ok(Json(RefreshResponse {
        access_token: outcome.access.token,
        token_type: "Bearer".into(),
        expires_in: outcome.access.expires_in,
        refresh_token: Some(outcome.refresh_token),
    }))
}

/// `POST /api/v1/auth/logout`
pub async fn logout(
    State(state): State<AppState>,
    Json(body): Json<LogoutRequest>,
) -> Result<axum::http::StatusCode, ApiError> {
    state.identity.logout(body.refresh_token).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// `GET /api/v1/auth/me`
///
/// TODO(Phase 1.3): extract `AuthContext` (user_id/tenant_id/roles) from the
/// verified JWT via middleware, then call `identity.me(&ctx.user_id)`.
pub async fn me(State(_state): State<AppState>) -> Result<Json<UserSummary>, ApiError> {
    Err(ApiError::not_implemented(
        "GET /me requires the JWT auth middleware (Phase 1.3)",
    ))
}
