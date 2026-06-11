//! Event-bus adapters (NATS today; Kafka/PubSub can be added behind the same
//! domain `RawEventPublisher` port later) + the outbox relay worker.

pub mod nats;
pub mod relay;
