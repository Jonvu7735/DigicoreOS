//! `DealsProjection` backed by Postgres (`reporting_svc.deal_facts`).
//!
//! `DealCreated` inserts with `ON CONFLICT (deal_id) DO NOTHING` (idempotent).
//! `DealStageChanged` updates the stage **only when the event is at least as
//! recent** as the stored `updated_at` (`$4 >= updated_at`), so a re-ordered or
//! duplicated change never regresses the funnel.

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::deals::entities::{DealStageChange, NewDealFact, StageCount};
use crate::domain::deals::ports::DealsProjection;
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;
use crate::infra::db::{map_db_err, map_write_err};

pub struct PgDealsRepo {
    pool: PgPool,
}

impl PgDealsRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DealsProjection for PgDealsRepo {
    async fn apply_deal_created(&self, fact: &NewDealFact) -> DomainResult<()> {
        sqlx::query(
            "INSERT INTO deal_facts \
             (deal_id, tenant_id, customer_id, amount_estimate, stage, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $6) ON CONFLICT (deal_id) DO NOTHING",
        )
        .bind(&fact.deal_id)
        .bind(&fact.tenant_id.0)
        .bind(&fact.customer_id)
        .bind(fact.amount_estimate)
        .bind(&fact.stage)
        .bind(fact.occurred_at)
        .execute(&self.pool)
        .await
        .map_err(map_write_err)?;
        Ok(())
    }

    async fn apply_deal_stage_changed(&self, change: &DealStageChange) -> DomainResult<()> {
        // Monotonic by event time: ignore changes older than what we've applied.
        sqlx::query(
            "UPDATE deal_facts SET stage = $3, updated_at = $4 \
             WHERE deal_id = $1 AND tenant_id = $2 AND $4 >= updated_at",
        )
        .bind(&change.deal_id)
        .bind(&change.tenant_id.0)
        .bind(&change.new_stage)
        .bind(change.occurred_at)
        .execute(&self.pool)
        .await
        .map_err(map_write_err)?;
        Ok(())
    }

    async fn funnel(&self, tenant: &TenantId) -> DomainResult<Vec<StageCount>> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT stage, count(*)::bigint FROM deal_facts \
             WHERE tenant_id = $1 GROUP BY stage ORDER BY stage",
        )
        .bind(&tenant.0)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows
            .into_iter()
            .map(|(stage, deal_count)| StageCount { stage, deal_count })
            .collect())
    }
}

#[cfg(test)]
mod db_integration {
    //! DB-backed tests; run only when `TEST_DATABASE_URL` is set (CI `integration`).

    use std::str::FromStr;

    use chrono::{Duration, Utc};
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use uuid::Uuid;

    use super::*;

    async fn pool_or_skip() -> Option<PgPool> {
        let url = std::env::var("TEST_DATABASE_URL").ok()?;
        let opts = PgConnectOptions::from_str(&url)
            .expect("valid TEST_DATABASE_URL")
            .options([("search_path", "reporting_svc")]);
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect_with(opts)
            .await
            .expect("connect to test db");
        crate::infra::db::postgres::run_migrations(&pool, "reporting_svc")
            .await
            .expect("apply migrations");
        Some(pool)
    }

    fn new_deal(tenant: &str, deal_id: &str, stage: &str) -> NewDealFact {
        NewDealFact {
            deal_id: deal_id.into(),
            tenant_id: TenantId(tenant.into()),
            customer_id: "c1".into(),
            amount_estimate: 1000,
            stage: stage.into(),
            occurred_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn funnel_counts_current_stage_and_changes_are_monotonic() {
        let Some(pool) = pool_or_skip().await else {
            return;
        };
        let repo = PgDealsRepo::new(pool);
        let tenant = TenantId(format!("it-{}", Uuid::now_v7()));
        let d1 = format!("d-{}", Uuid::now_v7());
        let d2 = format!("d-{}", Uuid::now_v7());

        repo.apply_deal_created(&new_deal(&tenant.0, &d1, "LEAD"))
            .await
            .unwrap();
        // Re-deliver: idempotent, no duplicate row / stage stays LEAD.
        repo.apply_deal_created(&new_deal(&tenant.0, &d1, "QUALIFIED"))
            .await
            .unwrap();
        repo.apply_deal_created(&new_deal(&tenant.0, &d2, "LEAD"))
            .await
            .unwrap();

        // Move d1 forward.
        repo.apply_deal_stage_changed(&DealStageChange {
            deal_id: d1.clone(),
            tenant_id: tenant.clone(),
            new_stage: "WON".into(),
            occurred_at: Utc::now(),
        })
        .await
        .unwrap();
        // A STALE change (older than what we applied) must be ignored.
        repo.apply_deal_stage_changed(&DealStageChange {
            deal_id: d1.clone(),
            tenant_id: tenant.clone(),
            new_stage: "LOST".into(),
            occurred_at: Utc::now() - Duration::hours(1),
        })
        .await
        .unwrap();

        let mut funnel = repo.funnel(&tenant).await.unwrap();
        funnel.sort_by(|a, b| a.stage.cmp(&b.stage));
        assert_eq!(funnel.len(), 2);
        // d2 still LEAD, d1 WON (the stale LOST change was ignored).
        assert_eq!(funnel[0].stage, "LEAD");
        assert_eq!(funnel[0].deal_count, 1);
        assert_eq!(funnel[1].stage, "WON");
        assert_eq!(funnel[1].deal_count, 1);
    }
}
