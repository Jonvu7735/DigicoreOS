//! Postgres-backed integration tests for the export vertical.
//!
//! Drives the real services over the real Pg repositories, so they exercise the
//! actual migrations and SQL (cargo lines, the in-transaction status history,
//! and the outbox write on each transition). Gated on `TEST_DATABASE_URL` (set
//! in CI's `vertical (trade-export)` job); skips otherwise so the default
//! `cargo test` stays infra-free.

use std::str::FromStr;
use std::sync::Arc;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::shared::types::TenantId;
use crate::domain::shipments::entities::ShipmentStatus;
use crate::domain::shipments::services::{NewCargoLine, ShipmentService};
use crate::infra::db::cargo_line_repo_pg::PgCargoLineRepo;
use crate::infra::db::shipment_repo_pg::PgShipmentRepo;
use crate::infra::time::clock::SystemClock;

async fn pool_or_skip() -> Option<PgPool> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let opts = PgConnectOptions::from_str(&url)
        .expect("valid TEST_DATABASE_URL")
        .options([("search_path", "trade_export_svc")]);
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect_with(opts)
        .await
        .expect("connect to test db");
    crate::infra::db::postgres::run_migrations(&pool, "trade_export_svc")
        .await
        .expect("apply migrations");
    Some(pool)
}

fn service(pool: &PgPool) -> ShipmentService {
    ShipmentService::new(
        Arc::new(PgShipmentRepo::new(pool.clone())),
        Arc::new(PgCargoLineRepo::new(pool.clone())),
        Arc::new(SystemClock),
    )
}

#[tokio::test]
async fn lifecycle_cargo_and_history_persist() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let svc = service(&pool);
    let tenant = TenantId(format!("it-{}", Uuid::now_v7()));

    let shipment = svc
        .create(&tenant, "US".into(), "FOB".into(), None)
        .await
        .unwrap();

    // Cargo lines persist and read back.
    svc.add_cargo_line(
        &tenant,
        &shipment.id,
        NewCargoLine {
            description: "Dried mango 500g".into(),
            hs_code: Some("08045000".into()),
            quantity: 1200,
            unit: "CTN".into(),
            net_weight_kg: Some(3600.0),
        },
    )
    .await
    .unwrap();
    svc.add_cargo_line(
        &tenant,
        &shipment.id,
        NewCargoLine {
            description: "Cashew W320".into(),
            hs_code: None,
            quantity: 800,
            unit: "CTN".into(),
            net_weight_kg: None,
        },
    )
    .await
    .unwrap();
    let cargo = svc.list_cargo_lines(&tenant, &shipment.id).await.unwrap();
    assert_eq!(cargo.len(), 2);
    assert_eq!(cargo[0].unit, "CTN");

    // Drive the lifecycle; each transition writes a history row + an outbox row.
    svc.book(&tenant, &shipment.id).await.unwrap();
    let dispatched = svc.dispatch(&tenant, &shipment.id).await.unwrap();
    assert_eq!(dispatched.status, ShipmentStatus::Dispatched);

    let history = svc
        .list_status_history(&tenant, &shipment.id)
        .await
        .unwrap();
    let trail: Vec<(Option<ShipmentStatus>, ShipmentStatus)> = history
        .iter()
        .map(|c| (c.from_status, c.to_status))
        .collect();
    assert_eq!(
        trail,
        vec![
            (None, ShipmentStatus::Draft),
            (Some(ShipmentStatus::Draft), ShipmentStatus::Booked),
            (Some(ShipmentStatus::Booked), ShipmentStatus::Dispatched),
        ]
    );

    // After DISPATCHED the manifest is frozen.
    assert!(svc
        .add_cargo_line(
            &tenant,
            &shipment.id,
            NewCargoLine {
                description: "late".into(),
                hs_code: None,
                quantity: 1,
                unit: "PCS".into(),
                net_weight_kg: None,
            },
        )
        .await
        .is_err());
}

#[tokio::test]
async fn draft_from_order_is_idempotent_in_db() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let svc = service(&pool);
    let tenant = TenantId(format!("it-{}", Uuid::now_v7()));
    let order = format!("ord-{}", Uuid::now_v7());

    assert!(svc
        .draft_from_order(&tenant, &order)
        .await
        .unwrap()
        .is_some());
    // The (tenant, order) unique index makes redelivery a no-op.
    assert!(svc
        .draft_from_order(&tenant, &order)
        .await
        .unwrap()
        .is_none());
}
