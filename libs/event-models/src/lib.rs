//! # event-models
//!
//! Shared business-event contracts for the DigicoreOS platform.
//!
//! Governing doc: `EVENTS.md` (event naming, subjects, header schema).
//! Rules:
//! - Event type names are PascalCase, past tense (`OrderCreated`, `UserRegistered`).
//! - NATS subjects follow `platform.<domain>.<entity>.<action_pasttense>`.
//! - Every event embeds an [`EventHeader`].
//! - Events are append-only: never change an existing payload in a breaking way –
//!   add a new `version` instead.

pub mod header;

pub mod auth;
pub mod erp;
pub mod crm;
pub mod hrm;
pub mod ai;

pub use header::EventHeader;
