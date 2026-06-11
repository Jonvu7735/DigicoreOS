//! HTTP middleware / extractors.
//!
//! `auth_context::Auth` is re-exported for handlers once the first guarded
//! route lands; the extractor itself is wired and ready.

pub mod auth_context;
