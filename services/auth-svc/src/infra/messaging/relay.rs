//! Outbox relay worker: drains `auth_svc.outbox_events` to NATS
//! (DATA-STRATEGY.md §3.2). Publish-then-mark gives at-least-once delivery;
//! consumers dedupe on `event_id`.

use std::sync::Arc;
use std::time::Duration;

use crate::domain::identity::ports::{OutboxRepository, RawEventPublisher};
use crate::domain::shared::error::DomainResult;

pub struct OutboxRelay {
    outbox: Arc<dyn OutboxRepository>,
    publisher: Arc<dyn RawEventPublisher>,
    poll_interval: Duration,
    batch_size: i64,
}

impl OutboxRelay {
    pub fn new(outbox: Arc<dyn OutboxRepository>, publisher: Arc<dyn RawEventPublisher>) -> Self {
        Self {
            outbox,
            publisher,
            poll_interval: Duration::from_secs(2),
            batch_size: 100,
        }
    }

    /// Run forever (spawn as a background task): drain, sleep, repeat.
    pub async fn run(self) {
        let mut ticker = tokio::time::interval(self.poll_interval);
        loop {
            ticker.tick().await;
            if let Err(error) = self.drain_once().await {
                tracing::warn!(%error, "outbox relay drain failed; will retry");
            }
        }
    }

    /// Publish one batch of unpublished messages; returns how many were sent.
    /// Stops the batch on the first publish error so ordering is preserved and
    /// the failed message is retried next tick.
    pub async fn drain_once(&self) -> DomainResult<usize> {
        let messages = self.outbox.fetch_unpublished(self.batch_size).await?;
        let mut sent = 0;
        for msg in messages {
            self.publisher
                .publish(&msg.subject, &msg.payload_bytes())
                .await?;
            self.outbox.mark_published(&msg.event_id).await?;
            metrics::counter!(
                "events_published_total",
                "service" => "auth-svc",
                "event_type" => msg.event_type.clone(),
            )
            .increment(1);
            tracing::info!(subject = %msg.subject, event_type = %msg.event_type, "outbox event published");
            sent += 1;
        }
        Ok(sent)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;
    use crate::domain::identity::outbox::OutboxMessage;
    use crate::domain::shared::error::DomainError;

    fn message(subject: &str) -> OutboxMessage {
        OutboxMessage {
            event_id: Uuid::now_v7(),
            occurred_at: Utc::now(),
            tenant_id: "t1".into(),
            aggregate_type: "user".into(),
            aggregate_id: "u1".into(),
            event_type: "UserRegistered".into(),
            version: 1,
            subject: subject.into(),
            payload: serde_json::json!({ "hello": "world" }),
        }
    }

    #[derive(Default)]
    struct FakeOutbox {
        pending: Mutex<Vec<OutboxMessage>>,
        marked: Mutex<Vec<Uuid>>,
    }
    #[async_trait]
    impl OutboxRepository for FakeOutbox {
        async fn fetch_unpublished(&self, _limit: i64) -> DomainResult<Vec<OutboxMessage>> {
            Ok(self.pending.lock().unwrap().clone())
        }
        async fn mark_published(&self, event_id: &Uuid) -> DomainResult<()> {
            self.marked.lock().unwrap().push(*event_id);
            self.pending
                .lock()
                .unwrap()
                .retain(|m| m.event_id != *event_id);
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakePublisher {
        sent: Mutex<Vec<(String, Vec<u8>)>>,
        fail: bool,
    }
    #[async_trait]
    impl RawEventPublisher for FakePublisher {
        async fn publish(&self, subject: &str, payload: &[u8]) -> DomainResult<()> {
            if self.fail {
                return Err(DomainError::Internal("publish failed".into()));
            }
            self.sent
                .lock()
                .unwrap()
                .push((subject.to_string(), payload.to_vec()));
            Ok(())
        }
    }

    #[tokio::test]
    async fn drains_and_marks_published() {
        let outbox = Arc::new(FakeOutbox::default());
        outbox.pending.lock().unwrap().extend([
            message("platform.auth.user.registered"),
            message("platform.auth.tenant.created"),
        ]);
        let publisher = Arc::new(FakePublisher::default());
        let relay = OutboxRelay::new(outbox.clone(), publisher.clone());

        let sent = relay.drain_once().await.unwrap();

        assert_eq!(sent, 2);
        assert_eq!(publisher.sent.lock().unwrap().len(), 2);
        assert_eq!(outbox.marked.lock().unwrap().len(), 2);
        assert!(outbox.pending.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn publish_failure_leaves_message_unmarked() {
        let outbox = Arc::new(FakeOutbox::default());
        outbox
            .pending
            .lock()
            .unwrap()
            .push(message("platform.auth.user.registered"));
        let publisher = Arc::new(FakePublisher {
            fail: true,
            ..Default::default()
        });
        let relay = OutboxRelay::new(outbox.clone(), publisher);

        assert!(relay.drain_once().await.is_err());
        // Nothing marked; the message stays for the next tick.
        assert!(outbox.marked.lock().unwrap().is_empty());
        assert_eq!(outbox.pending.lock().unwrap().len(), 1);
    }
}
