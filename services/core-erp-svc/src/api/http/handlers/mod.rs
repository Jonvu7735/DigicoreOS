//! HTTP handlers: parse DTO -> call domain -> map result/DomainError to DTO.

pub mod health;
pub mod metrics;
pub mod orders;
pub mod payments;
pub mod products;
