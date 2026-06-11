//! Pure business logic for auth-svc.
//!
//! HARD RULE (AI-FIRST-ARCHITECTURE.md): this module must NOT depend on
//! `api`, `infra`, or `utils`, and must NOT import axum, sqlx, async-nats,
//! tracing/config, or any other IO crate. Only data crates (serde-free here,
//! uuid/chrono for ids & time types), `thiserror`, `async-trait`, and the
//! shared `event-models` contract crate are allowed.

pub mod shared;

// Bounded context: identity (users, tenants, roles, permissions, tokens).
pub mod identity;
