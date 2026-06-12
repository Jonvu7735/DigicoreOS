//! Domain layer: pure business logic (no HTTP/SQL). `shared` holds primitives;
//! `loyalty` is the bounded context; `ingest` projects inbound core events.

pub mod ingest;
pub mod loyalty;
pub mod shared;
