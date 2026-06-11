//! `RefreshTokenRepository` implementation backed by Postgres
//! (`auth_svc.refresh_tokens`). Only token hashes are stored (SECURITY.md).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::identity::entities::RefreshToken;
use crate::domain::identity::ports::RefreshTokenRepository;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::{TenantId, UserId};
use crate::infra::db::{map_db_err, map_write_err};

type RefreshRow = (
    Uuid,
    Uuid,
    String,
    String,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    DateTime<Utc>,
);

fn to_refresh(r: RefreshRow) -> RefreshToken {
    RefreshToken {
        id: r.0,
        user_id: UserId(r.1),
        tenant_id: TenantId(r.2),
        token_hash: r.3,
        expires_at: r.4,
        revoked_at: r.5,
        created_at: r.6,
    }
}

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
    async fn insert(&self, token: &RefreshToken) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO refresh_tokens \
             (id, user_id, tenant_id, token_hash, expires_at, revoked_at, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(token.id)
        .bind(token.user_id.0)
        .bind(&token.tenant_id.0)
        .bind(&token.token_hash)
        .bind(token.expires_at)
        .bind(token.revoked_at)
        .bind(token.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_write_err)?;
        Ok(())
    }

    async fn find_valid_by_hash(&self, token_hash: &str) -> DomainResult<Option<RefreshToken>> {
        let row: Option<RefreshRow> = sqlx::query_as(
            "SELECT id, user_id, tenant_id, token_hash, expires_at, revoked_at, created_at \
             FROM refresh_tokens \
             WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > now()",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(row.map(to_refresh))
    }

    async fn revoke(&self, token_hash: &str) -> DomainResult<()> {
        // Idempotent: revoking an unknown/already-revoked token is a no-op.
        sqlx::query("UPDATE refresh_tokens SET revoked_at = now() WHERE token_hash = $1 AND revoked_at IS NULL")
            .bind(token_hash)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }
}
