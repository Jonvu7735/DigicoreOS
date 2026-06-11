//! # platform-events
//!
//! Shared inbound event-bus consumer for every event-CONSUMING service
//! (reporting-svc, ai-svc, …) — the subscriber-side mirror of `platform-outbox`:
//! - [`InboundEventHandler`]: the port a service implements (decode + project).
//! - [`NatsConsumer`]: background worker subscribing to `platform.>` and
//!   dispatching each message to the handler.
//! - [`connect_consumer`]: connect to NATS (the consumer runs only when reachable).
//!
//! Decoding the wire payload is the service's job (it owns the `event-models`
//! contracts), so this crate stays event-agnostic.

pub mod consumer;
pub mod error;
pub mod ports;

pub use consumer::{connect_consumer, NatsConsumer};
pub use error::{HandlerError, HandlerResult};
pub use ports::InboundEventHandler;
