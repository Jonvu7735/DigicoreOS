//! SHA-256 implementation of the domain `RefreshTokenHasher` port.
//!
//! The opaque token is two random UUIDv4s (~244 bits of entropy) rendered as
//! hex; we persist only its SHA-256 hash (SECURITY.md: never store raw tokens).

use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::domain::identity::ports::RefreshTokenHasher;

#[derive(Default)]
pub struct Sha256RefreshTokenHasher;

impl RefreshTokenHasher for Sha256RefreshTokenHasher {
    fn generate_opaque(&self) -> String {
        format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
    }

    fn hash(&self, raw: &str) -> String {
        let digest = Sha256::digest(raw.as_bytes());
        hex::encode(digest)
    }
}
