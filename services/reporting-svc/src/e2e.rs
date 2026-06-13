//! End-to-end test of the event backbone.
//!
//! Proves the architectural promise that a business event published by one
//! service reaches another service's read model: an `OrderCreated` written to a
//! transactional outbox flows
//!   outbox -> OutboxRelay -> NATS -> reporting NatsConsumer -> EventIngestor
//!   -> PgOrdersRepo (`order_facts`),
//! and the outbox row ends up marked published. This is the real wiring (the
//! same relay, consumer, ingestor and projections the service runs), not mocks.
//!
//! Gated on BOTH `TEST_DATABASE_URL` and `TEST_NATS_URL` (set in CI's
//! `integration` job); skips otherwise so the default `cargo test` stays
//! infra-free.

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use event_models::erp::{subjects, OrderCreated};
use event_models::EventHeader;
use platform_events::{connect_consumer, InboundEventHandler, NatsConsumer};
use platform_outbox::{
    connect_publisher, insert_outbox, OutboxMessage, OutboxRelay, OutboxRepository, PgOutboxRepo,
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::ingest::ingestor::EventIngestor;
use crate::domain::orders::ports::OrdersProjection;
use crate::domain::shared::types::TenantId;
use crate::infra::db::attendance_repo_pg::PgAttendanceRepo;
use crate::infra::db::customers_repo_pg::PgCustomersRepo;
use crate::infra::db::deals_repo_pg::PgDealsRepo;
use crate::infra::db::employees_repo_pg::PgEmployeesRepo;
use crate::infra::db::inventory_repo_pg::PgInventoryRepo;
use crate::infra::db::orders_repo_pg::PgOrdersRepo;
use crate::infra::db::sales_repo_pg::PgSalesRepo;

async fn pool_or_skip() -> Option<PgPool> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let opts = PgConnectOptions::from_str(&url)
        .expect("valid TEST_DATABASE_URL")
        .options([("search_path", "reporting_svc")]);
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect_with(opts)
        .await
        .expect("connect to test db");
    crate::infra::db::postgres::run_migrations(&pool, "reporting_svc")
        .await
        .expect("apply migrations");
    Some(pool)
}

/// Build the real reporting ingestor over a single pool (all seven projections,
/// exactly as `bootstrap::wiring` does).
fn ingestor(pool: &PgPool) -> (Arc<dyn InboundEventHandler>, Arc<dyn OrdersProjection>) {
    let orders: Arc<dyn OrdersProjection> = Arc::new(PgOrdersRepo::new(pool.clone()));
    let handler: Arc<dyn InboundEventHandler> = Arc::new(EventIngestor::new(
        Arc::new(PgSalesRepo::new(pool.clone())),
        orders.clone(),
        Arc::new(PgCustomersRepo::new(pool.clone())),
        Arc::new(PgEmployeesRepo::new(pool.clone())),
        Arc::new(PgDealsRepo::new(pool.clone())),
        Arc::new(PgInventoryRepo::new(pool.clone())),
        Arc::new(PgAttendanceRepo::new(pool.clone())),
    ));
    (handler, orders)
}

/// An `OrderCreated` queued as a producing service's outbox would write it (the
/// wire payload is the inner event struct, published on the ERP subject).
fn order_created_outbox(tenant: &str, order_id: &str, total: i64) -> OutboxMessage {
    let event_id = Uuid::now_v7();
    let now = Utc::now();
    let order = OrderCreated {
        header: EventHeader::new(
            event_id,
            now,
            tenant.to_string(),
            "order",
            order_id.to_string(),
            "OrderCreated",
            1,
        ),
        order_id: order_id.into(),
        customer_id: "c1".into(),
        total_amount: total,
        currency: "USD".into(),
        status: "NEW".into(),
    };
    OutboxMessage {
        event_id,
        occurred_at: now,
        tenant_id: tenant.into(),
        aggregate_type: "order".into(),
        aggregate_id: order_id.into(),
        event_type: "OrderCreated".into(),
        version: 1,
        subject: subjects::ORDER_CREATED.into(),
        payload: serde_json::to_value(&order).expect("serialize OrderCreated"),
    }
}

#[tokio::test]
async fn order_created_flows_outbox_relay_nats_consumer_to_orders_read_model() {
    let Some(pool) = pool_or_skip().await else {
        return; // no TEST_DATABASE_URL
    };
    let Ok(nats_url) = std::env::var("TEST_NATS_URL") else {
        return; // no TEST_NATS_URL — skip (default cargo test stays infra-free)
    };
    let Some(publisher) = connect_publisher(Some(&nats_url)).await else {
        return; // NATS unreachable
    };
    let Some(client) = connect_consumer(Some(&nats_url)).await else {
        return;
    };

    let tenant = TenantId(format!("e2e-{}", Uuid::now_v7()));
    let order_id = format!("o-{}", Uuid::now_v7());

    // 1) A producing service writes OrderCreated into its transactional outbox.
    let msg = order_created_outbox(&tenant.0, &order_id, 4242);
    let event_id = msg.event_id;
    let mut tx = pool.begin().await.unwrap();
    insert_outbox(&mut tx, &msg).await.unwrap();
    tx.commit().await.unwrap();

    // 2) Start the reporting consumer. JetStream is durable, so order vs. the
    //    relay no longer matters (the consumer replays the stream); a unique
    //    durable name keeps each test run's cursor isolated.
    let (handler, orders) = ingestor(&pool);
    let durable = format!("e2e-{}", Uuid::now_v7());
    let consumer = tokio::spawn(NatsConsumer::new(client, handler, durable).run());
    tokio::time::sleep(Duration::from_secs(1)).await;

    // 3) Start the outbox relay; it publishes the queued row to NATS.
    let outbox_repo: Arc<dyn OutboxRepository> = Arc::new(PgOutboxRepo::new(pool.clone()));
    let relay = tokio::spawn(OutboxRelay::new(outbox_repo, publisher).run());

    // 4) Eventually-consistent: poll the read model until the order lands.
    let mut projected = false;
    for _ in 0..100 {
        let listed = orders.list(&tenant, None, None, 50, 0).await.unwrap();
        if listed
            .iter()
            .any(|o| o.order_id == order_id && o.total_amount.0 == 4242)
        {
            projected = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    consumer.abort();
    relay.abort();

    assert!(
        projected,
        "OrderCreated did not reach the orders read model via outbox -> NATS -> consumer"
    );

    // The relay marked the outbox row published (at-least-once delivery done).
    let published_at: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT published_at FROM outbox_events WHERE id = $1")
            .bind(event_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        published_at.is_some(),
        "the relay should have marked the outbox row published"
    );
}
