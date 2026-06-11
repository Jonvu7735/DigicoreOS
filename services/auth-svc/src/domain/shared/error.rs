//! Domain error model (AI-FIRST-ARCHITECTURE.md §7.3).
//!
//! `api/http/dto/error.rs` maps these onto HTTP status codes + the standard
//! JSON error body. Domain code never deals with HTTP semantics.

/// Result alias used by all domain ports and services.
pub type DomainResult<T> = Result<T, DomainError>;

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    /// Entity does not exist (-> HTTP 404).
    #[error("{0} not found")]
    NotFound(String),

    /// Input failed business validation (-> HTTP 400).
    #[error("validation failed: {0}")]
    Validation(String),

    /// Caller is not authenticated / credentials invalid (-> HTTP 401).
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// Caller lacks the required permission (-> HTTP 403).
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// State conflict, e.g. duplicate email (-> HTTP 409).
    #[error("conflict: {0}")]
    Conflict(String),

    /// Unexpected infrastructure/internal failure (-> HTTP 500).
    /// Infra adapters wrap their errors into this variant.
    #[error("internal error: {0}")]
    Internal(String),
}
