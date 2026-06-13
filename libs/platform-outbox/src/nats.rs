//! JetStream publisher for the outbox relay.
//!
//! Publishing to JetStream (rather than core NATS) makes delivery durable: the
//! server persists each event to the `platform` stream and only ACKs once it is
//! stored, so an event published while a consumer is offline is **kept and
//! replayed** instead of silently dropped. The relay therefore only marks an
//! outbox row published after a server ACK — true end-to-end at-least-once.
//!
//! Each event also carries a `Nats-Msg-Id` (the outbox event id). Within the
//! stream's dedup window the server drops a re-published id, so a relay that
//! crashes between publish and mark — or two relay instances racing — cannot
//! put a duplicate on the bus.

use std::sync::Arc;
use std::time::Duration;

use async_nats::jetstream;
use async_trait::async_trait;
use uuid::Uuid;

use crate::message::{OutboxError, OutboxResult};
use crate::ports::RawEventPublisher;

/// JetStream stream that durably stores every platform business event. Created
/// idempotently by both the producer and the consumer side so neither depends
/// on the other's startup order (mirrored in `platform-events`).
pub const STREAM_NAME: &str = "platform";
/// Subject filter the stream captures (every `platform.<domain>.<entity>.<action>`).
pub const STREAM_SUBJECTS: &str = "platform.>";
/// Re-published event ids are deduplicated by the server for this window.
const DUPLICATE_WINDOW: Duration = Duration::from_secs(120);
/// Header carrying the dedup key (the outbox event id).
const MSG_ID_HEADER: &str = "Nats-Msg-Id";

/// Stream replication factor, from `JETSTREAM_REPLICAS` (default 1). Production
/// runs a 3-node JetStream cluster and sets this to 3 for HA; dev/CI use a
/// single node and leave it at 1. Clamped to JetStream's supported 1..=5.
pub(crate) fn stream_replicas() -> usize {
    std::env::var("JETSTREAM_REPLICAS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1)
        .clamp(1, 5)
}

pub struct JetStreamPublisher {
    context: jetstream::Context,
}

impl JetStreamPublisher {
    pub fn new(context: jetstream::Context) -> Self {
        Self { context }
    }
}

#[async_trait]
impl RawEventPublisher for JetStreamPublisher {
    async fn publish(&self, subject: &str, payload: &[u8], msg_id: &Uuid) -> OutboxResult<()> {
        let mut headers = async_nats::HeaderMap::new();
        headers.insert(MSG_ID_HEADER, msg_id.to_string());

        let ack = self
            .context
            .publish_with_headers(subject.to_string(), headers, payload.to_vec().into())
            .await
            .map_err(|e| OutboxError::Publish(e.to_string()))?;

        // Await the server ACK: only once the event is durably stored may the
        // relay mark the outbox row published.
        ack.await.map_err(|e| OutboxError::Publish(e.to_string()))?;
        Ok(())
    }
}

/// Ensure the durable `platform` stream exists (idempotent).
pub async fn ensure_stream(context: &jetstream::Context) -> Result<(), String> {
    context
        .get_or_create_stream(jetstream::stream::Config {
            name: STREAM_NAME.to_string(),
            subjects: vec![STREAM_SUBJECTS.to_string()],
            duplicate_window: DUPLICATE_WINDOW,
            num_replicas: stream_replicas(),
            ..Default::default()
        })
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Connect to NATS JetStream if configured & reachable, ensuring the stream
/// exists. The relay only runs when this is `Some`; otherwise events accumulate
/// in the outbox until a relay drains them.
pub async fn connect_publisher(nats_url: Option<&str>) -> Option<Arc<dyn RawEventPublisher>> {
    let Some(url) = nats_url else {
        tracing::warn!("NATS_URL not set; outbox relay disabled (events queue in the outbox)");
        return None;
    };
    match async_nats::connect(url).await {
        Ok(client) => {
            let context = jetstream::new(client);
            if let Err(error) = ensure_stream(&context).await {
                tracing::warn!(nats_url = url, %error, "JetStream stream unavailable; outbox relay disabled");
                return None;
            }
            tracing::info!(nats_url = url, "connected to NATS JetStream (publisher)");
            Some(Arc::new(JetStreamPublisher::new(context)))
        }
        Err(error) => {
            tracing::warn!(nats_url = url, %error, "NATS unreachable; outbox relay disabled");
            None
        }
    }
}
