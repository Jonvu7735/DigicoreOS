//! HTTP handlers, grouped by concern. Handlers: parse DTO -> call domain
//! service -> map result/DomainError to DTO/ApiError. No business logic here.

pub mod auth;
pub mod health;
pub mod metrics;
