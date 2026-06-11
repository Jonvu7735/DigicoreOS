//! The inbound-event handler port, called by [`crate::NatsConsumer`] for every
//! message on `platform.>`. Services implement it (decode + project).

use async_trait::async_trait;

use crate::error::HandlerResult;

#[async_trait]
pub trait InboundEventHandler: Send + Sync {
    /// Handle one raw bus message. Unrecognised subjects should be a no-op (Ok).
    async fn handle(&self, subject: &str, payload: &[u8]) -> HandlerResult<()>;
}
