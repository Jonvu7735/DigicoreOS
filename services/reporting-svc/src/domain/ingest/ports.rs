//! The inbound-event handler port, called by the NATS consumer (infra/messaging)
//! for every message on `platform.>`.

use async_trait::async_trait;

use crate::domain::shared::error::DomainResult;

#[async_trait]
pub trait InboundEventHandler: Send + Sync {
    /// Handle one raw bus message. Unrecognised subjects are ignored (Ok).
    async fn handle(&self, subject: &str, payload: &[u8]) -> DomainResult<()>;
}
