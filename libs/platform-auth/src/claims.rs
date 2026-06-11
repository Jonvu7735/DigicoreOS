//! Access-token claims (AUTH-FLOW.md §3). Shared so every service decodes the
//! same shape. Do NOT change the claim set without updating AUTH-FLOW.md.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    /// Subject – user id (UUID string).
    pub sub: String,
    pub tenant_id: String,
    pub roles: Vec<String>,
    pub iat: i64,
    pub exp: i64,
    pub iss: String,
    pub aud: String,
}
