//! NATS raw publisher used by the outbox relay (DATA-STRATEGY.md §3.2).

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::identity::ports::RawEventPublisher;
use crate::domain::shared::error::{DomainError, DomainResult};

/// Publishes raw `(subject, payload)` messages to NATS.
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
    async fn publish(&self, subject: &str, payload: &[u8]) -> DomainResult<()> {
        self.client
            .publish(subject.to_string(), Vec::from(payload).into())
            .await
            .map_err(|e| DomainError::Internal(format!("nats publish failed: {e}")))?;
        Ok(())
    }
}

/// Connect to NATS if configured & reachable. The outbox relay only runs when
/// this returns `Some`; otherwise events simply accumulate in the outbox until
/// a relay with a reachable broker drains them.
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
