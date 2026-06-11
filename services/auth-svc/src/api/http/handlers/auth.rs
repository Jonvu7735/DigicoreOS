//! Authentication & sign-up handlers (AUTH-FLOW.md §4–6, API-GATEWAY.md §3.1).
//!
//! Pattern for every handler: deserialize DTO -> call `IdentityService` ->
//! map `LoginOutcome`/`DomainError` to a response DTO / `ApiError`. Logging and
//! metrics live here so the domain service stays free of those concerns.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::api::http::dto::auth::{
    LoginRequest, LoginResponse, LogoutRequest, RefreshRequest, RefreshResponse, RegisterRequest,
    UserSummary,
};
use crate::api::http::dto::error::ApiError;
use crate::api::http::middleware::AuthContext;
use crate::bootstrap::wiring::AppState;
use crate::domain::identity::services::LoginOutcome;
use crate::domain::shared::types::{Email, TenantId};

/// Map a successful `LoginOutcome` (login or register) to the wire response.
fn login_response(outcome: LoginOutcome) -> LoginResponse {
    LoginResponse {
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
    }
}

/// `POST /api/v1/auth/login`
///
/// Emits `auth_login_success_total` / `auth_login_failed_total` and an INFO log
/// (no password/email/token – OBSERVABILITY.md §3.4) here in the api layer so
/// the domain service stays free of logging/metrics concerns.
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    match state
        .identity
        .login(
            Email(body.email),
            body.password,
            body.tenant_id.map(TenantId),
        )
        .await
    {
        Ok(outcome) => {
            metrics::counter!("auth_login_success_total", "service" => "auth-svc").increment(1);
            tracing::info!(user_id = %outcome.user.id, tenant_id = %outcome.tenant_id, "login successful");
            Ok(Json(login_response(outcome)))
        }
        Err(err) => {
            metrics::counter!("auth_login_failed_total", "service" => "auth-svc").increment(1);
            tracing::info!("login failed");
            Err(err.into())
        }
    }
}

/// `POST /api/v1/auth/register` – self-serve sign-up. Provisions a tenant + its
/// owner and returns a session (201). Public endpoint (no auth).
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<LoginResponse>), ApiError> {
    let outcome = state
        .identity
        .register(
            body.tenant_name,
            body.plan,
            Email(body.email),
            body.password,
            body.display_name,
        )
        .await?;
    metrics::counter!("auth_register_success_total", "service" => "auth-svc").increment(1);
    tracing::info!(user_id = %outcome.user.id, tenant_id = %outcome.tenant_id, "tenant registered");
    Ok((StatusCode::CREATED, Json(login_response(outcome))))
}

/// `POST /api/v1/auth/refresh`
pub async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, ApiError> {
    match state.identity.refresh(body.refresh_token).await {
        Ok(outcome) => {
            metrics::counter!("auth_refresh_success_total", "service" => "auth-svc").increment(1);
            Ok(Json(RefreshResponse {
                access_token: outcome.access.token,
                token_type: "Bearer".into(),
                expires_in: outcome.access.expires_in,
                refresh_token: Some(outcome.refresh_token),
            }))
        }
        Err(err) => {
            metrics::counter!("auth_refresh_failed_total", "service" => "auth-svc").increment(1);
            Err(err.into())
        }
    }
}

/// `POST /api/v1/auth/logout`
pub async fn logout(
    State(state): State<AppState>,
    Json(body): Json<LogoutRequest>,
) -> Result<StatusCode, ApiError> {
    state.identity.logout(body.refresh_token).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/v1/auth/me` – profile of the authenticated caller.
///
/// `AuthContext` (the extractor) verifies the bearer JWT; the active tenant and
/// roles come from the token, the profile from the user store.
pub async fn me(
    State(state): State<AppState>,
    ctx: AuthContext,
) -> Result<Json<UserSummary>, ApiError> {
    let user = state.identity.me(&ctx.user_id).await?;
    Ok(Json(UserSummary {
        id: user.id.to_string(),
        email: user.email.to_string(),
        display_name: user.display_name,
        tenant_id: ctx.tenant_id.0,
        roles: ctx.roles,
    }))
}
