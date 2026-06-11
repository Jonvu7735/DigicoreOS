//! # platform-outbox
//!
//! Transactional outbox mechanics shared by every event-publishing service
//! (DATA-STRATEGY.md §3.2):
//! - [`OutboxMessage`]: a queued business event (built from a service's event enum).
//! - [`insert_outbox`]: write one message inside the service's state transaction.
//! - [`OutboxRepository`] / [`PgOutboxRepo`]: read/clear side for the relay.
//! - [`RawEventPublisher`] / [`NatsRawPublisher`]: ship messages to the bus.
//! - [`OutboxRelay`]: background worker (publish-then-mark = at-least-once).
//!
//! Each service owns its own `outbox_events` table in its own schema; the pool's
//! `search_path` selects it.

pub mod message;
pub mod nats;
pub mod pg;
pub mod ports;
pub mod relay;

pub use message::{OutboxError, OutboxMessage, OutboxResult};
pub use nats::{connect_publisher, NatsRawPublisher};
pub use pg::{insert_outbox, PgOutboxRepo};
pub use ports::{OutboxRepository, RawEventPublisher};
pub use relay::OutboxRelay;
