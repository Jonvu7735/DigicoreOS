//! `LoyaltyRepository` backed by Postgres (`retail_svc.loyalty_accounts`).
//! Accrual is idempotent via `processed_events`; redeem writes state + an outbox
//! event in one transaction.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use platform_outbox::{insert_outbox, OutboxMessage};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::loyalty::entities::{LoyaltyAccount, PointsEntryKind, PointsLedgerEntry};
use crate::domain::loyalty::ports::LoyaltyRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::TenantId;
use crate::infra::db::{map_db_err, map_write_err};

type AccountRow = (String, String, i64, i64, DateTime<Utc>);

fn to_account(r: AccountRow) -> LoyaltyAccount {
    LoyaltyAccount {
        tenant_id: TenantId(r.0),
        customer_id: r.1,
        points_balance: r.2,
        lifetime_spend_minor: r.3,
        updated_at: r.4,
    }
}

const COLS: &str = "tenant_id, customer_id, points_balance, lifetime_spend_minor, updated_at";

type LedgerRow = (
    Uuid,
    String,
    String,
    String,
    i64,
    i64,
    Option<String>,
    DateTime<Utc>,
);

fn to_ledger(r: LedgerRow) -> DomainResult<PointsLedgerEntry> {
    let kind = PointsEntryKind::parse(&r.3)
        .ok_or_else(|| DomainError::Internal(format!("unknown ledger kind: {}", r.3)))?;
    Ok(PointsLedgerEntry {
        id: r.0,
        tenant_id: TenantId(r.1),
        customer_id: r.2,
        kind,
        points: r.4,
        balance_after: r.5,
        reason: r.6,
        at: r.7,
    })
}

const LEDGER_COLS: &str = "id, tenant_id, customer_id, kind, points, balance_after, reason, at";

fn outbox_err(e: platform_outbox::OutboxError) -> DomainError {
    DomainError::Internal(e.to_string())
}

/// Insert a points-ledger row on the given connection (used inside the same
/// transaction as the balance change it records).
async fn insert_ledger_entry(
    conn: &mut sqlx::PgConnection,
    entry: &PointsLedgerEntry,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO loyalty_points_ledger \
         (id, tenant_id, customer_id, kind, points, balance_after, reason, at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(entry.id)
    .bind(&entry.tenant_id.0)
    .bind(&entry.customer_id)
    .bind(entry.kind.as_str())
    .bind(entry.points)
    .bind(entry.balance_after)
    .bind(&entry.reason)
    .bind(entry.at)
    .execute(conn)
    .await?;
    Ok(())
}

pub struct PgLoyaltyRepo {
    pool: PgPool,
}

impl PgLoyaltyRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LoyaltyRepository for PgLoyaltyRepo {
    async fn accrue(
        &self,
        event_id: Uuid,
        entry_id: Uuid,
        tenant: &TenantId,
        customer_id: &str,
        spend_minor: i64,
        points: i64,
        reason: Option<&str>,
        now: DateTime<Utc>,
    ) -> DomainResult<bool> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;

        // Idempotency gate: first writer for this event_id wins; a redelivery is a
        // no-op. ON CONFLICT keeps it race-safe under concurrent consumers.
        let inserted = sqlx::query(
            "INSERT INTO processed_events (event_id, processed_at) VALUES ($1, $2) \
             ON CONFLICT (event_id) DO NOTHING",
        )
        .bind(event_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_write_err)?;

        if inserted.rows_affected() == 0 {
            // Already processed — commit the empty tx and report "not applied".
            tx.commit().await.map_err(map_db_err)?;
            return Ok(false);
        }

        // Upsert and read back the resulting balance for the ledger entry.
        let (balance_after,): (i64,) = sqlx::query_as(
            "INSERT INTO loyalty_accounts \
             (tenant_id, customer_id, points_balance, lifetime_spend_minor, updated_at) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (tenant_id, customer_id) DO UPDATE SET \
               points_balance = loyalty_accounts.points_balance + EXCLUDED.points_balance, \
               lifetime_spend_minor = loyalty_accounts.lifetime_spend_minor + EXCLUDED.lifetime_spend_minor, \
               updated_at = EXCLUDED.updated_at \
             RETURNING points_balance",
        )
        .bind(&tenant.0)
        .bind(customer_id)
        .bind(points)
        .bind(spend_minor)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_write_err)?;

        // A zero-point accrual still records the spend but is not a points
        // movement, so it doesn't belong in the points ledger.
        if points > 0 {
            let entry = PointsLedgerEntry {
                id: entry_id,
                tenant_id: tenant.clone(),
                customer_id: customer_id.to_string(),
                kind: PointsEntryKind::Earn,
                points,
                balance_after,
                reason: reason.map(str::to_string),
                at: now,
            };
            insert_ledger_entry(&mut tx, &entry)
                .await
                .map_err(map_write_err)?;
        }

        tx.commit().await.map_err(map_db_err)?;
        Ok(true)
    }

    async fn find(
        &self,
        tenant: &TenantId,
        customer_id: &str,
    ) -> DomainResult<Option<LoyaltyAccount>> {
        let row: Option<AccountRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM loyalty_accounts WHERE tenant_id = $1 AND customer_id = $2"
        ))
        .bind(&tenant.0)
        .bind(customer_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(row.map(to_account))
    }

    async fn list_in_tenant(
        &self,
        tenant: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<LoyaltyAccount>> {
        let rows: Vec<AccountRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM loyalty_accounts WHERE tenant_id = $1 \
             ORDER BY lifetime_spend_minor DESC LIMIT $2 OFFSET $3"
        ))
        .bind(&tenant.0)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(to_account).collect())
    }

    async fn save_balance(
        &self,
        account: &LoyaltyAccount,
        entry: &PointsLedgerEntry,
        event: &OutboxMessage,
    ) -> DomainResult<()> {
        let mut tx = self.pool.begin().await.map_err(map_db_err)?;
        sqlx::query(
            "UPDATE loyalty_accounts SET points_balance = $3, updated_at = $4 \
             WHERE tenant_id = $1 AND customer_id = $2",
        )
        .bind(&account.tenant_id.0)
        .bind(&account.customer_id)
        .bind(account.points_balance)
        .bind(account.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(map_write_err)?;
        insert_ledger_entry(&mut tx, entry)
            .await
            .map_err(map_write_err)?;
        insert_outbox(&mut tx, event).await.map_err(outbox_err)?;
        tx.commit().await.map_err(map_db_err)?;
        Ok(())
    }

    async fn list_ledger(
        &self,
        tenant: &TenantId,
        customer_id: &str,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<PointsLedgerEntry>> {
        let rows: Vec<LedgerRow> = sqlx::query_as(&format!(
            "SELECT {LEDGER_COLS} FROM loyalty_points_ledger \
             WHERE tenant_id = $1 AND customer_id = $2 \
             ORDER BY at DESC, id DESC LIMIT $3 OFFSET $4"
        ))
        .bind(&tenant.0)
        .bind(customer_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.into_iter().map(to_ledger).collect()
    }
}
