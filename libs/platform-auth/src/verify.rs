//! RS256 verification of access tokens (AUTH-FLOW.md §7). Verifiers hold only
//! the public key; issuance lives in `auth-svc`.

use anyhow::Context;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

use crate::claims::AccessTokenClaims;

/// Authn/authz failures mapped by each service onto its HTTP error type.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
}

/// Verifies RS256 access tokens against a public key, issuer and audience.
pub struct JwtVerifier {
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtVerifier {
    pub fn from_public_key_pem(pem: &str, issuer: &str, audience: &str) -> anyhow::Result<Self> {
        let decoding_key = DecodingKey::from_rsa_pem(pem.as_bytes())
            .context("invalid RSA public key PEM (JWT verification)")?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[issuer]);
        validation.set_audience(&[audience]);
        Ok(Self {
            decoding_key,
            validation,
        })
    }

    pub fn verify(&self, token: &str) -> Result<AccessTokenClaims, AuthError> {
        decode::<AccessTokenClaims>(token, &self.decoding_key, &self.validation)
            .map(|data| data.claims)
            .map_err(|e| AuthError::Unauthorized(format!("invalid access token: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use rsa::pkcs1::EncodeRsaPrivateKey;
    use rsa::pkcs8::EncodePublicKey;
    use rsa::RsaPrivateKey;

    use super::*;

    fn keypair() -> (String, String) {
        let mut rng = rand::thread_rng();
        let key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
        (
            key.to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
                .unwrap()
                .to_string(),
            key.to_public_key()
                .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
                .unwrap(),
        )
    }

    fn sign(private_pem: &str, claims: &AccessTokenClaims) -> String {
        let key = EncodingKey::from_rsa_pem(private_pem.as_bytes()).unwrap();
        encode(&Header::new(Algorithm::RS256), claims, &key).unwrap()
    }

    fn claims() -> AccessTokenClaims {
        let now = 1_700_000_000;
        AccessTokenClaims {
            sub: "u1".into(),
            tenant_id: "t1".into(),
            roles: vec!["OWNER".into()],
            iat: now,
            exp: now + 1_000_000_000,
            iss: "auth-svc".into(),
            aud: "platform-api".into(),
        }
    }

    #[test]
    fn verifies_valid_token() {
        let (priv_pem, pub_pem) = keypair();
        let token = sign(&priv_pem, &claims());
        let verifier =
            JwtVerifier::from_public_key_pem(&pub_pem, "auth-svc", "platform-api").unwrap();
        let decoded = verifier.verify(&token).unwrap();
        assert_eq!(decoded.tenant_id, "t1");
        assert_eq!(decoded.roles, vec!["OWNER".to_string()]);
    }

    #[test]
    fn rejects_token_from_another_key() {
        let (priv_pem, _) = keypair();
        let (_, other_pub) = keypair();
        let token = sign(&priv_pem, &claims());
        let verifier =
            JwtVerifier::from_public_key_pem(&other_pub, "auth-svc", "platform-api").unwrap();
        assert!(verifier.verify(&token).is_err());
    }

    #[test]
    fn rejects_garbage() {
        let (_, pub_pem) = keypair();
        let verifier =
            JwtVerifier::from_public_key_pem(&pub_pem, "auth-svc", "platform-api").unwrap();
        assert!(verifier.verify("not.a.jwt").is_err());
    }
}
