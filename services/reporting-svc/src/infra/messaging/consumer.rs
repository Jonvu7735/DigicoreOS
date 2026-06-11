//! NATS consumer: subscribes to `platform.>` and feeds every message to the
//! domain [`InboundEventHandler`] (OBSERVABILITY.md: events_consumed_total).
//!
//! Generic mechanics — when ai-svc also needs to consume, this can be extracted
//! into a shared `platform-events` crate (cf. how `platform-outbox` was lifted
//! out of the producer side).

use std::sync::Arc;

use futures::StreamExt;

use crate::domain::ingest::ports::InboundEventHandler;

/// Wildcard subject covering every platform event.
const PLATFORM_SUBJECT: &str = "platform.>";

pub struct NatsConsumer {
    client: async_nats::Client,
    handler: Arc<dyn InboundEventHandler>,
}

impl NatsConsumer {
    pub fn new(client: async_nats::Client, handler: Arc<dyn InboundEventHandler>) -> Self {
        Self { client, handler }
    }

    /// Run forever (spawn as a background task): drain the subscription and
    /// apply each event. A handler error is logged and skipped so one bad
    /// message can't stall the stream.
    pub async fn run(self) {
        let mut subscription = match self.client.subscribe(PLATFORM_SUBJECT).await {
            Ok(sub) => sub,
            Err(error) => {
                tracing::error!(%error, "failed to subscribe to platform events");
                return;
            }
        };
        tracing::info!(subject = PLATFORM_SUBJECT, "event consumer subscribed");

        while let Some(message) = subscription.next().await {
            let subject = message.subject.as_str();
            match self.handler.handle(subject, message.payload.as_ref()).await {
                Ok(()) => {
                    metrics::counter!("events_consumed_total", "subject" => subject.to_string())
                        .increment(1);
                }
                Err(error) => {
                    metrics::counter!("events_consumed_failed_total").increment(1);
                    tracing::warn!(%error, subject = %subject, "failed to apply inbound event");
                }
            }
        }
        tracing::warn!("event consumer subscription ended");
    }
}

/// Connect to NATS if configured & reachable. The consumer only runs when this
/// returns `Some`; otherwise reporting serves whatever read-model state exists.
pub async fn connect_consumer(nats_url: Option<&str>) -> Option<async_nats::Client> {
    match nats_url {
        Some(url) => match async_nats::connect(url).await {
            Ok(client) => {
                tracing::info!(nats_url = url, "event consumer connected to NATS");
                Some(client)
            }
            Err(error) => {
                tracing::warn!(nats_url = url, %error, "NATS unreachable; event consumer disabled");
                None
            }
        },
        None => {
            tracing::warn!("NATS_URL not set; event consumer disabled");
            None
        }
    }
}
