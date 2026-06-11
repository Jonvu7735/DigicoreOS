//! `AuthContext`: the authenticated caller + permission checks against the
//! shared RBAC matrix. Each service wraps this in its own Axum extractor.

use uuid::Uuid;

use crate::claims::AccessTokenClaims;
use crate::rbac;
use crate::verify::AuthError;

/// Authenticated caller derived from verified JWT claims.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub tenant_id: String,
    pub roles: Vec<String>,
}

impl AuthContext {
    /// Build from verified claims (`sub` must be a UUID user id).
    pub fn from_claims(claims: AccessTokenClaims) -> Result<Self, AuthError> {
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AuthError::Unauthorized("invalid token subject".into()))?;
        Ok(Self {
            user_id,
            tenant_id: claims.tenant_id,
            roles: claims.roles,
        })
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// True if any of the caller's roles grants `permission` (shared matrix).
    pub fn has_permission(&self, permission: &str) -> bool {
        self.roles
            .iter()
            .any(|role| rbac::permissions_for(role).contains(&permission))
    }

    /// `Ok(())` if the caller holds `permission`, else `PermissionDenied`.
    pub fn require_permission(&self, permission: &str) -> Result<(), AuthError> {
        if self.has_permission(permission) {
            Ok(())
        } else {
            Err(AuthError::PermissionDenied(format!(
                "missing permission: {permission}"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(sub: &str, roles: &[&str]) -> AccessTokenClaims {
        AccessTokenClaims {
            sub: sub.into(),
            tenant_id: "t1".into(),
            roles: roles.iter().map(|r| r.to_string()).collect(),
            iat: 0,
            exp: 0,
            iss: "auth-svc".into(),
            aud: "platform-api".into(),
        }
    }

    #[test]
    fn from_claims_parses_uuid() {
        let id = Uuid::now_v7();
        let ctx = AuthContext::from_claims(claims(&id.to_string(), &["OWNER"])).unwrap();
        assert_eq!(ctx.user_id, id);
        assert!(ctx.has_role("OWNER"));
    }

    #[test]
    fn from_claims_rejects_bad_subject() {
        assert!(AuthContext::from_claims(claims("nope", &["OWNER"])).is_err());
    }

    #[test]
    fn require_permission_uses_shared_matrix() {
        let owner =
            AuthContext::from_claims(claims(&Uuid::now_v7().to_string(), &["OWNER"])).unwrap();
        assert!(owner.require_permission("erp_order_create").is_ok());
        let viewer =
            AuthContext::from_claims(claims(&Uuid::now_v7().to_string(), &["VIEWER"])).unwrap();
        assert!(viewer.has_permission("erp_order_read"));
        assert!(viewer.require_permission("erp_order_create").is_err());
    }
}
