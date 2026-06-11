//! JWT implementation of the `TokenIssuer` port (AUTH-FLOW.md §3).
//!
//! Skeleton uses HS256 with a shared secret. TODO(Phase 1.3): switch to RS256
//! (private key here, public key distributed to gateway/services) per
//! AUTH-FLOW.md, and expose a JWKS endpoint if needed.

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::bootstrap::config::JwtConfig;
use crate::domain::identity::ports::{AccessTokenClaims, IssuedToken, TokenIssuer};
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::{TenantId, UserId};

/// Wire-format claims (serde mirror of the domain `AccessTokenClaims`).
#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    sub: String,
    tenant_id: String,
    roles: Vec<String>,
    iat: i64,
    exp: i64,
    iss: String,
    aud: String,
}

pub struct JwtTokenIssuer {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    issuer: String,
    audience: String,
    access_ttl_secs: i64,
}

impl JwtTokenIssuer {
    pub fn from_config(config: &JwtConfig) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(config.hs256_secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(config.hs256_secret.as_bytes()),
            issuer: config.issuer.clone(),
            audience: config.audience.clone(),
            access_ttl_secs: config.access_ttl_secs,
        }
    }
}

impl TokenIssuer for JwtTokenIssuer {
    fn issue_access_token(
        &self,
        user_id: &UserId,
        tenant_id: &TenantId,
        roles: &[String],
        now: DateTime<Utc>,
    ) -> DomainResult<IssuedToken> {
        let exp = now + Duration::seconds(self.access_ttl_secs);
        let claims = JwtClaims {
            sub: user_id.to_string(),
            tenant_id: tenant_id.to_string(),
            roles: roles.to_vec(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| DomainError::Internal(format!("jwt encode failed: {e}")))?;

        Ok(IssuedToken {
            token,
            expires_in: self.access_ttl_secs,
        })
    }

    fn validate_access_token(&self, token: &str) -> DomainResult<AccessTokenClaims> {
        let mut validation = Validation::default(); // HS256
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_audience(&[self.audience.as_str()]);

        let data = decode::<JwtClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| DomainError::Unauthorized(format!("invalid access token: {e}")))?;

        let c = data.claims;
        Ok(AccessTokenClaims {
            sub: c.sub,
            tenant_id: c.tenant_id,
            roles: c.roles,
            iat: c.iat,
            exp: c.exp,
            iss: c.iss,
            aud: c.aud,
        })
    }
}
