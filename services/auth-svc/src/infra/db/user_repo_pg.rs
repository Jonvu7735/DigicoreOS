//! `UserRepository` implementation backed by Postgres (`auth_svc.users`).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::identity::entities::User;
use crate::domain::identity::ports::UserRepository;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::{Email, UserId};
use crate::infra::db::{map_db_err, map_write_err};

/// Column tuple for `users` rows, mapped to the domain `User`.
type UserRow = (Uuid, String, String, String, bool, DateTime<Utc>);

fn to_user(r: UserRow) -> User {
    User {
        id: UserId(r.0),
        email: Email(r.1),
        display_name: r.2,
        password_hash: r.3,
        is_active: r.4,
        created_at: r.5,
    }
}

const SELECT_COLS: &str = "id, email, display_name, password_hash, is_active, created_at";

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
    async fn find_by_id(&self, id: &UserId) -> DomainResult<Option<User>> {
        let row: Option<UserRow> =
            sqlx::query_as(&format!("SELECT {SELECT_COLS} FROM users WHERE id = $1"))
                .bind(id.0)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_db_err)?;
        Ok(row.map(to_user))
    }

    async fn find_by_email(&self, email: &Email) -> DomainResult<Option<User>> {
        let row: Option<UserRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLS} FROM users WHERE lower(email) = lower($1)"
        ))
        .bind(&email.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(row.map(to_user))
    }

    async fn insert(&self, user: &User) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO users (id, email, display_name, password_hash, is_active, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(user.id.0)
        .bind(&user.email.0)
        .bind(&user.display_name)
        .bind(&user.password_hash)
        .bind(user.is_active)
        .bind(user.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_write_err)?;
        Ok(())
    }

    async fn update(&self, user: &User) -> DomainResult<()> {
        sqlx::query(
            "UPDATE users SET email = $2, display_name = $3, password_hash = $4, \
             is_active = $5 WHERE id = $1",
        )
        .bind(user.id.0)
        .bind(&user.email.0)
        .bind(&user.display_name)
        .bind(&user.password_hash)
        .bind(user.is_active)
        .execute(&self.pool)
        .await
        .map_err(map_write_err)?;
        Ok(())
    }
}
