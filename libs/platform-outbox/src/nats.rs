//! NATS raw publisher for the outbox relay.

use std::sync::Arc;

use async_trait::async_trait;

use crate::message::{OutboxError, OutboxResult};
use crate::ports::RawEventPublisher;

pub struct NatsRawPublisher {
    client: async_nats::Client,
}

impl NatsRawPublisher {
    pub fn new(client: async_nats::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl RawEventPublisher for NatsRawPublisher {
    async fn publish(&self, subject: &str, payload: &[u8]) -> OutboxResult<()> {
        self.client
            .publish(subject.to_string(), Vec::from(payload).into())
            .await
            .map_err(|e| OutboxError::Publish(e.to_string()))?;
        Ok(())
    }
}

/// Connect to NATS if configured & reachable. The relay only runs when this is
/// `Some`; otherwise events accumulate in the outbox until a relay drains them.
pub async fn connect_publisher(nats_url: Option<&str>) -> Option<Arc<dyn RawEventPublisher>> {
    match nats_url {
        Some(url) => match async_nats::connect(url).await {
            Ok(client) => {
                tracing::info!(nats_url = url, "connected to NATS");
                Some(Arc::new(NatsRawPublisher::new(client)))
            }
            Err(error) => {
                tracing::warn!(nats_url = url, %error, "NATS unreachable; outbox relay disabled");
                None
            }
        },
        None => {
            tracing::warn!("NATS_URL not set; outbox relay disabled (events queue in the outbox)");
            None
        }
    }
}
