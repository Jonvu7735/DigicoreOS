//! # platform-auth
//!
//! Shared authentication/authorization toolkit for DigicoreOS services
//! (AUTH-FLOW.md, SECURITY.md). Provides:
//! - [`rbac`]: the platform role->permission matrix (single source of truth).
//! - [`JwtVerifier`]: RS256 verification of the access tokens `auth-svc` issues.
//! - [`AccessTokenClaims`] / [`AuthContext`]: claims + permission guard.
//!
//! Token *issuance* (the private key) stays in `auth-svc`; every service
//! *verifies* with the public key and enforces the same matrix.

pub mod claims;
pub mod context;
pub mod rbac;
pub mod verify;

pub use claims::AccessTokenClaims;
pub use context::AuthContext;
pub use verify::{AuthError, JwtVerifier};
