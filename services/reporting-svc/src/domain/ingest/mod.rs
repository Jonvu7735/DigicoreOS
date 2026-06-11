//! Inbound event ingestion: decode platform events and update read models.
//! The `InboundEventHandler` port + NATS consumer live in `platform-events`.

pub mod ingestor;
