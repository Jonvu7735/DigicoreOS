//! NATS implementation of the domain `EventPublisher` port (EVENTS.md).
//!
//! TODO(Phase 1.5): replace direct publishing with the outbox pattern
//! (DATA-STRATEGY.md §3.2): domain writes events to `auth_svc.outbox_events`
//! in the same transaction as state; a relay worker publishes from the outbox
//! to NATS and marks rows as sent.

use std::sync::Arc;

use async_trait::async_trait;
use event_models::auth::AuthEvent;

use crate::domain::identity::ports::EventPublisher;
use crate::domain::shared::error::{DomainError, DomainResult};

/// Publishes events to NATS subjects defined in `event-models`.
pub struct NatsEventPublisher {
    client: async_nats::Client,
}

#[async_trait]
impl EventPublisher for NatsEventPublisher {
    async fn publish(&self, event: AuthEvent) -> DomainResult<()> {
        let subject = event.subject();
        let event_type = event.event_type();
        let payload = event
            .payload_json()
            .map_err(|e| DomainError::Internal(format!("event serialization failed: {e}")))?;

        self.client
            .publish(subject, payload.into())
            .await
            .map_err(|e| DomainError::Internal(format!("nats publish failed: {e}")))?;

        metrics::counter!(
            "events_published_total",
            "service" => "auth-svc",
            "event_type" => event_type,
        )
        .increment(1);

        tracing::info!(subject, event_type, "event published");
        Ok(())
    }
}

/// Dev fallback when NATS is not configured/reachable: logs and drops events
/// so local HTTP development is not blocked. NEVER used in staging/prod.
pub struct NoopEventPublisher;

#[async_trait]
impl EventPublisher for NoopEventPublisher {
    async fn publish(&self, event: AuthEvent) -> DomainResult<()> {
        tracing::warn!(
            subject = event.subject(),
            event_type = event.event_type(),
            "NoopEventPublisher: event DROPPED (NATS not configured)"
        );
        Ok(())
    }
}

/// Build the publisher for wiring: real NATS when configured & reachable,
/// otherwise the noop fallback (with a loud warning).
pub async fn build_publisher(nats_url: Option<&str>) -> Arc<dyn EventPublisher> {
    match nats_url {
        Some(url) => match async_nats::connect(url).await {
            Ok(client) => {
                tracing::info!(nats_url = url, "connected to NATS");
                Arc::new(NatsEventPublisher { client })
            }
            Err(error) => {
                tracing::warn!(nats_url = url, %error, "NATS unreachable; using NoopEventPublisher");
                Arc::new(NoopEventPublisher)
            }
        },
        None => {
            tracing::warn!("NATS_URL not set; using NoopEventPublisher");
            Arc::new(NoopEventPublisher)
        }
    }
}
