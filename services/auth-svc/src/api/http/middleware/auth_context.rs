//! `AuthContext` – authenticated caller derived from a verified access token
//! (AUTH-FLOW.md §7).
//!
//! As an Axum extractor it: (1) reads `Authorization: Bearer <jwt>`,
//! (2) validates it via the domain `TokenIssuer` (through `IdentityService`),
//! (3) records `user_id` / `tenant_id` on the current `http.server` span so
//! they appear in every JSON log line (OBSERVABILITY.md §3.2). Rejects with 401
//! when the header is missing or the token is invalid.

use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use uuid::Uuid;

use crate::api::http::dto::error::ApiError;
use crate::bootstrap::wiring::AppState;
use crate::domain::identity::ports::AccessTokenClaims;
use crate::domain::shared::error::DomainError;
use crate::domain::shared::types::{TenantId, UserId};
use platform_auth::rbac;

/// Authenticated request context extracted from a verified JWT.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub roles: Vec<String>,
}

impl AuthContext {
    /// True if the caller holds `role` in the active tenant.
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// True if any of the caller's roles grants `permission` under the default
    /// RBAC matrix. Roles are seeded from that matrix and per-role permissions
    /// are not yet editable, so it is authoritative for now (switch to a
    /// DB-backed lookup once role editing exists).
    pub fn has_permission(&self, permission: &str) -> bool {
        self.roles
            .iter()
            .any(|role| rbac::permissions_for(role).contains(&permission))
    }

    /// `Ok(())` if the caller holds `permission`, otherwise a 403 `ApiError`.
    pub fn require_permission(&self, permission: &str) -> Result<(), ApiError> {
        if self.has_permission(permission) {
            Ok(())
        } else {
            Err(DomainError::PermissionDenied(format!("missing permission: {permission}")).into())
        }
    }

    /// Build from validated JWT claims (`sub` must be a UUID user id).
    fn from_claims(claims: AccessTokenClaims) -> Result<Self, DomainError> {
        let user_id = Uuid::parse_str(&claims.sub)
            .map(UserId)
            .map_err(|_| DomainError::Unauthorized("invalid token subject".into()))?;
        Ok(Self {
            user_id,
            tenant_id: TenantId(claims.tenant_id),
            roles: claims.roles,
        })
    }
}

/// Extract the bearer token from an `Authorization` header value.
fn bearer_token(header: Option<&str>) -> Result<&str, DomainError> {
    let header =
        header.ok_or_else(|| DomainError::Unauthorized("missing Authorization header".into()))?;
    header
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| DomainError::Unauthorized("expected 'Bearer <token>'".into()))
}

impl FromRequestParts<AppState> for AuthContext {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok());
        let token = bearer_token(header)?;
        let claims = state.identity.validate_access_token(token)?;
        let ctx = AuthContext::from_claims(claims)?;

        // Surface identity on the request span (OBSERVABILITY.md §3.2/§5.4).
        let span = tracing::Span::current();
        span.record("user_id", tracing::field::display(&ctx.user_id));
        span.record("tenant_id", tracing::field::display(&ctx.tenant_id));

        Ok(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(sub: &str) -> AccessTokenClaims {
        AccessTokenClaims {
            sub: sub.into(),
            tenant_id: "t1".into(),
            roles: vec!["OWNER".into()],
            iat: 0,
            exp: 0,
            iss: "auth-svc".into(),
            aud: "platform-api".into(),
        }
    }

    #[test]
    fn bearer_token_parsing() {
        assert!(bearer_token(None).is_err());
        assert!(bearer_token(Some("Basic abc")).is_err());
        assert!(bearer_token(Some("Bearer ")).is_err());
        assert_eq!(
            bearer_token(Some("Bearer abc.def.ghi")).unwrap(),
            "abc.def.ghi"
        );
        assert_eq!(bearer_token(Some("Bearer   spaced  ")).unwrap(), "spaced");
    }

    #[test]
    fn from_claims_parses_uuid_subject_and_roles() {
        let id = Uuid::now_v7();
        let ctx = AuthContext::from_claims(claims(&id.to_string())).unwrap();
        assert_eq!(ctx.user_id.0, id);
        assert_eq!(ctx.tenant_id.0, "t1");
        assert!(ctx.has_role("OWNER"));
        assert!(!ctx.has_role("ADMIN"));
    }

    #[test]
    fn from_claims_rejects_non_uuid_subject() {
        assert!(AuthContext::from_claims(claims("not-a-uuid")).is_err());
    }

    fn ctx(roles: &[&str]) -> AuthContext {
        AuthContext {
            user_id: UserId(Uuid::now_v7()),
            tenant_id: TenantId("t1".into()),
            roles: roles.iter().map(|r| r.to_string()).collect(),
        }
    }

    #[test]
    fn require_permission_enforces_rbac_matrix() {
        // OWNER can do everything.
        assert!(ctx(&["OWNER"])
            .require_permission("auth_user_create")
            .is_ok());
        // VIEWER can read but not create users.
        let viewer = ctx(&["VIEWER"]);
        assert!(viewer.has_permission("auth_user_read"));
        assert!(!viewer.has_permission("auth_user_create"));
        assert!(viewer.require_permission("auth_user_create").is_err());
        // Multiple roles union their permissions.
        assert!(ctx(&["VIEWER", "ADMIN"])
            .require_permission("auth_user_create")
            .is_ok());
    }
}
