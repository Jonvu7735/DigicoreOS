//! Id generation helpers. UUIDv7 is time-ordered (index-friendly).

use uuid::Uuid;

/// Preferred id for new aggregates (time-ordered).
pub fn new_id() -> Uuid {
    Uuid::now_v7()
}

/// Random id, when ordering must not leak creation time.
pub fn new_random_id() -> Uuid {
    Uuid::new_v4()
}
