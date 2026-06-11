//! Outbox relay worker: drains `outbox_events` to the bus (DATA-STRATEGY.md
//! §3.2). Publish-then-mark = at-least-once; consumers dedupe on `event_id`.

use std::sync::Arc;
use std::time::Duration;

use crate::message::OutboxResult;
use crate::ports::{OutboxRepository, RawEventPublisher};

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

    /// Publish one batch; returns how many were sent. Stops on the first publish
    /// error so ordering holds and the failed message is retried next tick.
    pub async fn drain_once(&self) -> OutboxResult<usize> {
        let messages = self.outbox.fetch_unpublished(self.batch_size).await?;
        let mut sent = 0;
        for msg in messages {
            self.publisher
                .publish(&msg.subject, &msg.payload_bytes())
                .await?;
            self.outbox.mark_published(&msg.event_id).await?;
            metrics::counter!("events_published_total", "event_type" => msg.event_type.clone())
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
    use crate::message::{OutboxError, OutboxMessage};

    fn message(subject: &str) -> OutboxMessage {
        OutboxMessage {
            event_id: Uuid::now_v7(),
            occurred_at: Utc::now(),
            tenant_id: "t1".into(),
            aggregate_type: "order".into(),
            aggregate_id: "o1".into(),
            event_type: "OrderCreated".into(),
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
        async fn fetch_unpublished(&self, _limit: i64) -> OutboxResult<Vec<OutboxMessage>> {
            Ok(self.pending.lock().unwrap().clone())
        }
        async fn mark_published(&self, event_id: &Uuid) -> OutboxResult<()> {
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
        sent: Mutex<Vec<String>>,
        fail: bool,
    }
    #[async_trait]
    impl RawEventPublisher for FakePublisher {
        async fn publish(&self, subject: &str, _payload: &[u8]) -> OutboxResult<()> {
            if self.fail {
                return Err(OutboxError::Publish("boom".into()));
            }
            self.sent.lock().unwrap().push(subject.to_string());
            Ok(())
        }
    }

    #[tokio::test]
    async fn drains_and_marks_published() {
        let outbox = Arc::new(FakeOutbox::default());
        outbox.pending.lock().unwrap().extend([
            message("platform.erp.order.created"),
            message("platform.erp.order.paid"),
        ]);
        let publisher = Arc::new(FakePublisher::default());
        let relay = OutboxRelay::new(outbox.clone(), publisher.clone());

        assert_eq!(relay.drain_once().await.unwrap(), 2);
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
            .push(message("platform.erp.order.created"));
        let publisher = Arc::new(FakePublisher {
            fail: true,
            ..Default::default()
        });
        let relay = OutboxRelay::new(outbox.clone(), publisher);

        assert!(relay.drain_once().await.is_err());
        assert!(outbox.marked.lock().unwrap().is_empty());
        assert_eq!(outbox.pending.lock().unwrap().len(), 1);
    }
}
