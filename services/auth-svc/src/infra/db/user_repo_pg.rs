//! `UserRepository` implementation backed by Postgres (`auth_svc.users`).

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::identity::entities::User;
use crate::domain::identity::ports::UserRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::{Email, UserId};

pub struct PgUserRepo {
    pool: PgPool,
}

impl PgUserRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PgUserRepo {
    // TODO(Phase 1.2): SELECT id, email, display_name, password_hash,
    //   is_active, created_at FROM users WHERE id = $1
    async fn find_by_id(&self, _id: &UserId) -> DomainResult<Option<User>> {
        Err(DomainError::Internal(
            "PgUserRepo::find_by_id not implemented (Phase 1.2)".into(),
        ))
    }

    // TODO(Phase 1.2): SELECT ... FROM users WHERE lower(email) = lower($1)
    async fn find_by_email(&self, _email: &Email) -> DomainResult<Option<User>> {
        Err(DomainError::Internal(
            "PgUserRepo::find_by_email not implemented (Phase 1.2)".into(),
        ))
    }

    // TODO(Phase 1.2): INSERT INTO users (...) VALUES (...)
    //   ON CONFLICT (email) -> map to DomainError::Conflict.
    async fn insert(&self, _user: &User) -> DomainResult<()> {
        Err(DomainError::Internal(
            "PgUserRepo::insert not implemented (Phase 1.2)".into(),
        ))
    }

    // TODO(Phase 1.2): UPDATE users SET ... WHERE id = $1
    async fn update(&self, _user: &User) -> DomainResult<()> {
        Err(DomainError::Internal(
            "PgUserRepo::update not implemented (Phase 1.2)".into(),
        ))
    }
}
