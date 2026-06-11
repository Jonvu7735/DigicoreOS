//! Infrastructure adapters. Each submodule implements ports defined in
//! `domain/` – infra introduces NO new business behavior.

pub mod db;
pub mod messaging;
pub mod security;
pub mod time;
