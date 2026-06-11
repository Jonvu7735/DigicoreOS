//! RS256 implementation of the `TokenIssuer` port (AUTH-FLOW.md §3).
//!
//! The service signs with an RSA private key; the gateway and other services
//! verify with the matching public key only. Keys are PEM strings injected via
//! config (env/secret) – never committed (SECURITY.md). Claims are fixed:
//! `sub`, `tenant_id`, `roles`, `iat`, `exp`, `iss`, `aud` (do not change).

use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
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
    header: Header,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    issuer: String,
    audience: String,
    access_ttl_secs: i64,
}

impl JwtTokenIssuer {
    /// Build from PEM-encoded RSA keys. Fails fast on malformed key material.
    pub fn from_config(config: &JwtConfig) -> anyhow::Result<Self> {
        let encoding_key = EncodingKey::from_rsa_pem(config.private_key_pem.as_bytes())
            .context("invalid RSA private key PEM (JWT signing)")?;
        let decoding_key = DecodingKey::from_rsa_pem(config.public_key_pem.as_bytes())
            .context("invalid RSA public key PEM (JWT verification)")?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[config.issuer.as_str()]);
        validation.set_audience(&[config.audience.as_str()]);

        Ok(Self {
            header: Header::new(Algorithm::RS256),
            encoding_key,
            decoding_key,
            validation,
            issuer: config.issuer.clone(),
            audience: config.audience.clone(),
            access_ttl_secs: config.access_ttl_secs,
        })
    }
}

impl TokenIssuer for JwtTokenIssuer {
    #[tracing::instrument(skip_all, name = "generate_token")]
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

        let token = encode(&self.header, &claims, &self.encoding_key)
            .map_err(|e| DomainError::Internal(format!("jwt encode failed: {e}")))?;

        Ok(IssuedToken {
            token,
            expires_in: self.access_ttl_secs,
        })
    }

    fn validate_access_token(&self, token: &str) -> DomainResult<AccessTokenClaims> {
        let data = decode::<JwtClaims>(token, &self.decoding_key, &self.validation)
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

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use rsa::pkcs1::EncodeRsaPrivateKey;
    use rsa::pkcs8::EncodePublicKey;
    use rsa::RsaPrivateKey;
    use uuid::Uuid;

    use super::JwtTokenIssuer;
    use crate::bootstrap::config::JwtConfig;
    use crate::domain::identity::ports::TokenIssuer;
    use crate::domain::shared::types::{TenantId, UserId};

    /// Build a config with a freshly generated RSA key pair (no committed keys).
    fn test_config() -> JwtConfig {
        let mut rng = rand::thread_rng();
        let priv_key = RsaPrivateKey::new(&mut rng, 2048).expect("generate RSA key");
        let private_key_pem = priv_key
            .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
            .expect("encode private key")
            .to_string();
        let public_key_pem = priv_key
            .to_public_key()
            .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
            .expect("encode public key");
        JwtConfig {
            issuer: "auth-svc".into(),
            audience: "platform-api".into(),
            access_ttl_secs: 1800,
            refresh_ttl_secs: 1_209_600,
            private_key_pem,
            public_key_pem,
        }
    }

    #[test]
    fn issue_then_validate_round_trip() {
        let issuer = JwtTokenIssuer::from_config(&test_config()).unwrap();
        let issued = issuer
            .issue_access_token(
                &UserId(Uuid::now_v7()),
                &TenantId("t1".into()),
                &["OWNER".into(), "ADMIN".into()],
                Utc::now(),
            )
            .unwrap();
        assert_eq!(issued.expires_in, 1800);

        let claims = issuer.validate_access_token(&issued.token).unwrap();
        assert_eq!(claims.tenant_id, "t1");
        assert_eq!(claims.roles, vec!["OWNER".to_string(), "ADMIN".to_string()]);
        assert_eq!(claims.iss, "auth-svc");
        assert_eq!(claims.aud, "platform-api");
    }

    #[test]
    fn validate_rejects_garbage_token() {
        let issuer = JwtTokenIssuer::from_config(&test_config()).unwrap();
        assert!(issuer.validate_access_token("not.a.jwt").is_err());
    }

    #[test]
    fn validate_rejects_token_signed_by_another_key() {
        let signer = JwtTokenIssuer::from_config(&test_config()).unwrap();
        let issued = signer
            .issue_access_token(
                &UserId(Uuid::now_v7()),
                &TenantId("t1".into()),
                &[],
                Utc::now(),
            )
            .unwrap();
        // A verifier with a DIFFERENT public key must reject the token.
        let other = JwtTokenIssuer::from_config(&test_config()).unwrap();
        assert!(other.validate_access_token(&issued.token).is_err());
    }

    /// `scripts/gen-dev-jwt-keys.sh` emits a PKCS#8 private key + SPKI public key
    /// (openssl defaults); make sure those load (the round-trip test above uses
    /// PKCS#1, so this guards the dev-key path specifically).
    #[test]
    fn from_config_accepts_pkcs8_private_key() {
        use rsa::pkcs8::EncodePrivateKey;

        let mut rng = rand::thread_rng();
        let priv_key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let cfg = JwtConfig {
            issuer: "auth-svc".into(),
            audience: "platform-api".into(),
            access_ttl_secs: 1800,
            refresh_ttl_secs: 1_209_600,
            private_key_pem: priv_key
                .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
                .unwrap()
                .to_string(),
            public_key_pem: priv_key
                .to_public_key()
                .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
                .unwrap(),
        };

        let issuer = JwtTokenIssuer::from_config(&cfg).expect("PKCS#8 dev key should load");
        let issued = issuer
            .issue_access_token(
                &UserId(Uuid::now_v7()),
                &TenantId("t".into()),
                &[],
                Utc::now(),
            )
            .unwrap();
        assert!(issuer.validate_access_token(&issued.token).is_ok());
    }
}
