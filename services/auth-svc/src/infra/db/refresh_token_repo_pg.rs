//! `RefreshTokenRepository` implementation backed by Postgres
//! (`auth_svc.refresh_tokens`).

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::identity::entities::RefreshToken;
use crate::domain::identity::ports::RefreshTokenRepository;
use crate::domain::shared::error::{DomainError, DomainResult};

pub struct PgRefreshTokenRepo {
    pool: PgPool,
}

impl PgRefreshTokenRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RefreshTokenRepository for PgRefreshTokenRepo {
    // TODO(Phase 1.2): INSERT INTO refresh_tokens (...) VALUES (...)
    async fn insert(&self, _token: &RefreshToken) -> DomainResult<()> {
        Err(DomainError::Internal(
            "PgRefreshTokenRepo::insert not implemented (Phase 1.2)".into(),
        ))
    }

    // TODO(Phase 1.2): SELECT ... WHERE token_hash = $1
    //   AND revoked_at IS NULL AND expires_at > now()
    async fn find_valid_by_hash(&self, _token_hash: &str) -> DomainResult<Option<RefreshToken>> {
        Err(DomainError::Internal(
            "PgRefreshTokenRepo::find_valid_by_hash not implemented (Phase 1.2)".into(),
        ))
    }

    // TODO(Phase 1.2): UPDATE refresh_tokens SET revoked_at = now()
    //   WHERE token_hash = $1
    async fn revoke(&self, _token_hash: &str) -> DomainResult<()> {
        Err(DomainError::Internal(
            "PgRefreshTokenRepo::revoke not implemented (Phase 1.2)".into(),
        ))
    }
}
