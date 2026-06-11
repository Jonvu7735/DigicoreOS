//! Argon2 implementation of the domain `PasswordHasher` port
//! (AUTH-FLOW.md §4: verify password hash; SECURITY.md: never log/store raw
//! passwords).

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, SaltString};
use argon2::{Argon2, PasswordHasher as _, PasswordVerifier as _};

use crate::domain::identity::ports::PasswordHasher;
use crate::domain::shared::error::{DomainError, DomainResult};

#[derive(Default)]
pub struct Argon2PasswordHasher;

impl PasswordHasher for Argon2PasswordHasher {
    fn hash(&self, raw_password: &str) -> DomainResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(raw_password.as_bytes(), &salt)
            .map_err(|e| DomainError::Internal(format!("password hashing failed: {e}")))?;
        Ok(hash.to_string())
    }

    fn verify(&self, raw_password: &str, password_hash: &str) -> DomainResult<bool> {
        let parsed = PasswordHash::new(password_hash)
            .map_err(|e| DomainError::Internal(format!("stored password hash invalid: {e}")))?;
        Ok(Argon2::default()
            .verify_password(raw_password.as_bytes(), &parsed)
            .is_ok())
    }
}
