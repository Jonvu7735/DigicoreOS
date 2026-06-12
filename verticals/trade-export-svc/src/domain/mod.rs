//! Domain layer: pure business logic (no HTTP/SQL). `shared` holds primitives;
//! `shipments` is the bounded context; `ingest` projects inbound core events.

pub mod ingest;
pub mod shared;
pub mod shipments;
