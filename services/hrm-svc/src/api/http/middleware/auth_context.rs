//! `Auth` extractor: verifies the bearer JWT (via the shared platform-auth
//! verifier) and exposes the platform `AuthContext` to handlers. Records
//! `user_id`/`tenant_id` on the request span (OBSERVABILITY.md §3.2).

use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use platform_auth::{AuthContext, AuthError};

use crate::api::http::dto::error::ApiError;
use crate::bootstrap::wiring::AppState;

/// Local newtype so we can implement Axum's `FromRequestParts` for the foreign
/// `platform_auth::AuthContext` (orphan rule).
pub struct Auth(pub AuthContext);

fn bearer_token(header: Option<&str>) -> Result<&str, AuthError> {
    let header =
        header.ok_or_else(|| AuthError::Unauthorized("missing Authorization header".into()))?;
    header
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| AuthError::Unauthorized("expected 'Bearer <token>'".into()))
}

impl FromRequestParts<AppState> for Auth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok());
        let token = bearer_token(header).map_err(ApiError::from)?;
        let claims = state.verifier.verify(token).map_err(ApiError::from)?;
        let ctx = AuthContext::from_claims(claims).map_err(ApiError::from)?;

        let span = tracing::Span::current();
        span.record("user_id", tracing::field::display(&ctx.user_id));
        span.record("tenant_id", tracing::field::display(&ctx.tenant_id));

        Ok(Auth(ctx))
    }
}
