//! Error returned by an [`crate::InboundEventHandler`].

/// A handler failed to process a message. The consumer logs it and moves on
/// (the bus will redeliver per its own policy); services map their internal
/// errors into this at the handler boundary.
#[derive(Debug, thiserror::Error)]
#[error("inbound event handler failed: {0}")]
pub struct HandlerError(pub String);

pub type HandlerResult<T> = Result<T, HandlerError>;
