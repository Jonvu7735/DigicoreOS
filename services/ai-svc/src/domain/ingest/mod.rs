//! Inbound event ingestion: react to platform events by generating insights.
//! The `InboundEventHandler` port + NATS consumer live in `platform-events`.

pub mod ingestor;
