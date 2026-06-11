//! Postgres adapters for the `auth_svc` schema (DATA-STRATEGY.md §3.1).

pub mod postgres;
pub mod refresh_token_repo_pg;
pub mod role_repo_pg;
pub mod tenant_repo_pg;
pub mod user_repo_pg;
