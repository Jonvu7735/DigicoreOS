//! Wire-format DTOs (contracts-first, AI-FIRST-ARCHITECTURE.md §7.2).
//! DTOs are deliberately separate from domain entities – never serialize an
//! entity straight onto the wire.

pub mod auth;
pub mod error;
