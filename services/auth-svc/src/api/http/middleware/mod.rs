//! HTTP middleware.
//!
//! TODO(Phase 1.3) – `auth_context`: middleware/extractor that
//! 1. reads `Authorization: Bearer <jwt>`,
//! 2. validates it via the domain `TokenIssuer` port,
//! 3. exposes [`AuthContext`] to handlers,
//! 4. records `tenant_id`/`user_id` on the current `http.server` span so they
//!    appear in every JSON log line (OBSERVABILITY.md §3.2),
//! 5. rejects with 401 (invalid token) / 403 (missing permission) per
//!    AUTH-FLOW.md §7.

/// Authenticated request context extracted from a verified JWT.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub tenant_id: String,
    pub roles: Vec<String>,
}
