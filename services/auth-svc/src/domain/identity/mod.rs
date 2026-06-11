//! Bounded context: **identity**.
//!
//! Owns users, tenants, roles, permissions, refresh tokens, and the
//! login/refresh/logout use-cases (SERVICE-auth-svc.md, AUTH-FLOW.md).

pub mod entities;
pub mod outbox;
pub mod ports;
pub mod provisioning;
pub mod services;
