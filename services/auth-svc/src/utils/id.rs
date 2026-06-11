//! Id generation helpers.
//!
//! UUIDv7 is time-ordered, which keeps Postgres B-tree indexes append-friendly
//! for high-insert tables (users, refresh_tokens, outbox_events).

use uuid::Uuid;

/// Preferred id for new aggregates (time-ordered).
pub fn new_id() -> Uuid {
    Uuid::now_v7()
}

/// Random id, for cases where ordering must not leak creation time.
pub fn new_random_id() -> Uuid {
    Uuid::new_v4()
}
