//! HTTP layer (Axum). Talks to `domain` services only – never to the DB or
//! message broker directly (AI-FIRST-ARCHITECTURE.md dependency rules).

pub mod dto;
pub mod handlers;
pub mod middleware;
pub mod routes;
