//! Postgres-backed integration tests for the retail vertical.
//!
//! Drives the real LoyaltyService over the real Pg repository, exercising the
//! actual migrations and SQL: the idempotent accrue upsert (`RETURNING` the
//! balance), the in-transaction points ledger, and per-tenant rules. Gated on
//! `TEST_DATABASE_URL` (set in CI's `vertical (retail)` job); skips otherwise so
//! the default `cargo test` stays infra-free.

use std::str::FromStr;
use std::sync::Arc;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::loyalty::entities::LoyaltyRules;
use crate::domain::loyalty::services::LoyaltyService;
use crate::domain::shared::types::TenantId;
use crate::infra::db::loyalty_repo_pg::PgLoyaltyRepo;
use crate::infra::time::clock::SystemClock;

async fn pool_or_skip() -> Option<PgPool> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let opts = PgConnectOptions::from_str(&url)
        .expect("valid TEST_DATABASE_URL")
        .options([("search_path", "retail_svc")]);
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect_with(opts)
        .await
        .expect("connect to test db");
    crate::infra::db::postgres::run_migrations(&pool, "retail_svc")
        .await
        .expect("apply migrations");
    Some(pool)
}

fn service(pool: &PgPool) -> LoyaltyService {
    LoyaltyService::new(
        Arc::new(PgLoyaltyRepo::new(pool.clone())),
        Arc::new(SystemClock),
    )
}

#[tokio::test]
async fn accrue_redeem_rules_and_ledger_persist() {
    let Some(pool) = pool_or_skip().await else {
        return;
    };
    let svc = service(&pool);
    let tenant = TenantId(format!("it-{}", Uuid::now_v7()));
    let customer = format!("c-{}", Uuid::now_v7());

    // Configure a generous program (1 point / 50 minor units).
    svc.set_rules(
        &tenant,
        LoyaltyRules {
            minor_per_point: 50,
            silver_min: 100_000,
            gold_min: 1_000_000,
        },
    )
    .await
    .unwrap();
    assert_eq!(svc.rules(&tenant).await.unwrap().minor_per_point, 50);

    // Accrue under the tenant's rate; redelivery of the same event is a no-op.
    let event_id = Uuid::now_v7();
    assert!(svc
        .accrue_for_order(event_id, &tenant, &customer, "ord-1", 10_000)
        .await
        .unwrap());
    assert!(!svc
        .accrue_for_order(event_id, &tenant, &customer, "ord-1", 10_000)
        .await
        .unwrap());

    let account = svc.get(&tenant, &customer).await.unwrap();
    assert_eq!(account.points_balance, 200); // 10_000 / 50, credited once
    assert_eq!(account.lifetime_spend_minor, 10_000);

    svc.redeem(&tenant, &customer, 30).await.unwrap();
    let account = svc.get(&tenant, &customer).await.unwrap();
    assert_eq!(account.points_balance, 170);

    // Ledger has both movements, newest first, with running balances.
    let ledger = svc.list_ledger(&tenant, &customer, 50, 0).await.unwrap();
    let rows: Vec<(&str, i64, i64)> = ledger
        .iter()
        .map(|e| (e.kind.as_str(), e.points, e.balance_after))
        .collect();
    assert_eq!(rows, vec![("REDEEM", 30, 170), ("EARN", 200, 200)]);
}
