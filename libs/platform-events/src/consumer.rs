//! JetStream consumer: durably consumes `platform.>` and feeds every message to
//! an [`InboundEventHandler`] (OBSERVABILITY.md: events_consumed_total).
//!
//! Each consuming service gets its own **durable** consumer (named after the
//! service), so the server tracks its cursor independently and replays anything
//! it missed while offline — fixing the silent loss that core NATS suffered.
//!
//! Delivery is ACK-based: a handled message is ACKed; a transient failure is
//! NAKed (with backoff) so the server redelivers it. After [`MAX_DELIVER`]
//! attempts the message is parked on a dead-letter stream and TERMed, so one
//! poison message can neither be lost silently nor block the stream forever.

use std::sync::Arc;
use std::time::Duration;

use async_nats::jetstream::{self, consumer::pull, consumer::AckPolicy, AckKind};
use futures::StreamExt;

use crate::ports::InboundEventHandler;

/// Durable `platform` stream (mirrors `platform-outbox`; both create it idempotently).
const STREAM_NAME: &str = "platform";
const STREAM_SUBJECTS: &str = "platform.>";
/// Dead-letter stream for messages that exhausted their retries. It captures
/// `dlq.>` (NOT under `platform.>`), so dead letters are never re-consumed.
const DLQ_STREAM_NAME: &str = "platform_dlq";
const DLQ_SUBJECTS: &str = "dlq.>";
/// Total delivery attempts before a message is dead-lettered.
const MAX_DELIVER: i64 = 5;
/// How long the server waits for an ACK before redelivering.
const ACK_WAIT: Duration = Duration::from_secs(30);
/// Backoff applied when NAKing a transient failure.
const NAK_BACKOFF: Duration = Duration::from_secs(5);

/// Stream replication factor, from `JETSTREAM_REPLICAS` (default 1). Mirrors
/// `platform-outbox`: production's 3-node cluster sets 3 for HA; dev/CI use 1.
fn stream_replicas() -> usize {
    std::env::var("JETSTREAM_REPLICAS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1)
        .clamp(1, 5)
}

pub struct NatsConsumer {
    client: async_nats::Client,
    handler: Arc<dyn InboundEventHandler>,
    /// Durable consumer name — unique per consuming service.
    durable: String,
}

impl NatsConsumer {
    pub fn new(
        client: async_nats::Client,
        handler: Arc<dyn InboundEventHandler>,
        durable: impl Into<String>,
    ) -> Self {
        Self {
            client,
            handler,
            durable: durable.into(),
        }
    }

    /// Run forever (spawn as a background task): bind a durable consumer and
    /// apply each event, ACK/NAK/dead-letter per outcome.
    pub async fn run(self) {
        let context = jetstream::new(self.client.clone());

        // Both producer and consumer create the stream; ordering-independent.
        let stream = match context
            .get_or_create_stream(jetstream::stream::Config {
                name: STREAM_NAME.to_string(),
                subjects: vec![STREAM_SUBJECTS.to_string()],
                num_replicas: stream_replicas(),
                ..Default::default()
            })
            .await
        {
            Ok(stream) => stream,
            Err(error) => {
                tracing::error!(%error, "failed to ensure platform stream; consumer disabled");
                return;
            }
        };

        // Best-effort dead-letter stream (consuming still works without it).
        if let Err(error) = context
            .get_or_create_stream(jetstream::stream::Config {
                name: DLQ_STREAM_NAME.to_string(),
                subjects: vec![DLQ_SUBJECTS.to_string()],
                num_replicas: stream_replicas(),
                ..Default::default()
            })
            .await
        {
            tracing::warn!(%error, "dead-letter stream unavailable; failed messages will be TERMed without parking");
        }

        let consumer = match stream
            .get_or_create_consumer(
                &self.durable,
                pull::Config {
                    durable_name: Some(self.durable.clone()),
                    ack_policy: AckPolicy::Explicit,
                    ack_wait: ACK_WAIT,
                    max_deliver: MAX_DELIVER,
                    ..Default::default()
                },
            )
            .await
        {
            Ok(consumer) => consumer,
            Err(error) => {
                tracing::error!(%error, durable = %self.durable, "failed to bind durable consumer");
                return;
            }
        };

        let mut messages = match consumer.messages().await {
            Ok(messages) => messages,
            Err(error) => {
                tracing::error!(%error, "failed to open consumer message stream");
                return;
            }
        };
        tracing::info!(stream = STREAM_NAME, durable = %self.durable, "event consumer subscribed (JetStream)");

        while let Some(next) = messages.next().await {
            let message = match next {
                Ok(message) => message,
                Err(error) => {
                    tracing::warn!(%error, "error pulling next message");
                    continue;
                }
            };

            let subject = message.subject.to_string();
            match self.handler.handle(&subject, &message.payload).await {
                Ok(()) => {
                    metrics::counter!("events_consumed_total", "subject" => subject.clone())
                        .increment(1);
                    if let Err(error) = message.ack().await {
                        tracing::warn!(%error, subject = %subject, "failed to ACK message");
                    }
                }
                Err(error) => {
                    let delivered = message.info().map(|info| info.delivered).unwrap_or(1);
                    if delivered < MAX_DELIVER {
                        tracing::warn!(%error, subject = %subject, delivered, "handler failed; NAK for redelivery");
                        if let Err(ack_err) =
                            message.ack_with(AckKind::Nak(Some(NAK_BACKOFF))).await
                        {
                            tracing::warn!(error = %ack_err, "failed to NAK message");
                        }
                    } else {
                        // Retries exhausted: park on the DLQ, then TERM so the
                        // server stops redelivering. Loss is avoided (parked);
                        // the stream is never blocked by one poison message.
                        metrics::counter!("events_consumed_failed_total").increment(1);
                        let dlq_subject = format!("dlq.{subject}");
                        if let Err(pub_err) =
                            context.publish(dlq_subject, message.payload.clone()).await
                        {
                            tracing::error!(error = %pub_err, subject = %subject, "failed to dead-letter message");
                        }
                        tracing::error!(%error, subject = %subject, delivered, "handler failed permanently; dead-lettered");
                        if let Err(ack_err) = message.ack_with(AckKind::Term).await {
                            tracing::warn!(error = %ack_err, "failed to TERM message");
                        }
                    }
                }
            }
        }
        tracing::warn!("event consumer subscription ended");
    }
}

/// Connect to NATS if configured & reachable. The consumer only runs when this
/// returns `Some`; otherwise the service serves whatever state already exists.
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
