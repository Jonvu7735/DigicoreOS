//! Activity use-cases (event-less CRUD; logged against a customer).

use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::activities::entities::{Activity, ActivityKind};
use crate::domain::activities::ports::ActivityRepository;
use crate::domain::customers::ports::CustomerRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::{Clock, TenantId};

pub struct ActivityService {
    repo: Arc<dyn ActivityRepository>,
    customers: Arc<dyn CustomerRepository>,
    clock: Arc<dyn Clock>,
}

impl ActivityService {
    pub fn new(
        repo: Arc<dyn ActivityRepository>,
        customers: Arc<dyn CustomerRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repo,
            customers,
            clock,
        }
    }

    pub async fn log(
        &self,
        tenant_id: &TenantId,
        customer_id: Uuid,
        kind: String,
        subject: String,
        notes: Option<String>,
        occurred_at: Option<String>,
    ) -> DomainResult<Activity> {
        let kind = parse_kind(&kind)?;
        let subject = subject.trim().to_string();
        if subject.is_empty() {
            return Err(DomainError::Validation("subject is required".into()));
        }
        let occurred_at = parse_occurred_at(occurred_at, self.clock.now_utc())?;
        self.customers
            .find_in_tenant(tenant_id, &customer_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("customer {customer_id}")))?;

        let activity = Activity {
            id: Uuid::now_v7(),
            tenant_id: tenant_id.clone(),
            customer_id,
            kind,
            subject,
            notes: normalize(notes),
            occurred_at,
            created_at: self.clock.now_utc(),
        };
        self.repo.insert(&activity).await?;
        Ok(activity)
    }

    pub async fn list(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Activity>> {
        self.repo.list_in_tenant(tenant_id, limit, offset).await
    }

    pub async fn get(&self, tenant_id: &TenantId, id: &Uuid) -> DomainResult<Activity> {
        self.repo
            .find_in_tenant(tenant_id, id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("activity {id}")))
    }

    pub async fn update(
        &self,
        tenant_id: &TenantId,
        id: &Uuid,
        kind: Option<String>,
        subject: Option<String>,
        notes: Option<String>,
    ) -> DomainResult<Activity> {
        let mut activity = self.get(tenant_id, id).await?;
        if let Some(kind) = kind {
            activity.kind = parse_kind(&kind)?;
        }
        if let Some(subject) = subject {
            let subject = subject.trim().to_string();
            if subject.is_empty() {
                return Err(DomainError::Validation("subject cannot be empty".into()));
            }
            activity.subject = subject;
        }
        if let Some(notes) = notes {
            activity.notes = normalize(Some(notes));
        }
        self.repo.update(&activity).await?;
        Ok(activity)
    }
}

fn parse_kind(raw: &str) -> DomainResult<ActivityKind> {
    let norm = raw.trim().to_uppercase();
    ActivityKind::parse(&norm)
        .ok_or_else(|| DomainError::Validation(format!("unknown activity kind: {raw}")))
}

fn parse_occurred_at(value: Option<String>, default: DateTime<Utc>) -> DomainResult<DateTime<Utc>> {
    match value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
    {
        None => Ok(default),
        Some(s) => DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| {
                DomainError::Validation("occurred_at must be an RFC3339 timestamp".into())
            }),
    }
}

/// Trim an optional free-text field; an empty string becomes `None`.
fn normalize(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use platform_outbox::OutboxMessage;

    use super::*;
    use crate::domain::customers::entities::Customer;

    #[derive(Default)]
    struct FakeActivities {
        items: Mutex<Vec<Activity>>,
    }
    #[async_trait]
    impl ActivityRepository for FakeActivities {
        async fn insert(&self, activity: &Activity) -> DomainResult<()> {
            self.items.lock().unwrap().push(activity.clone());
            Ok(())
        }
        async fn list_in_tenant(
            &self,
            _t: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<Activity>> {
            Ok(self.items.lock().unwrap().clone())
        }
        async fn find_in_tenant(&self, _t: &TenantId, id: &Uuid) -> DomainResult<Option<Activity>> {
            Ok(self
                .items
                .lock()
                .unwrap()
                .iter()
                .find(|a| a.id == *id)
                .cloned())
        }
        async fn update(&self, activity: &Activity) -> DomainResult<()> {
            let mut items = self.items.lock().unwrap();
            if let Some(slot) = items.iter_mut().find(|a| a.id == activity.id) {
                *slot = activity.clone();
            }
            Ok(())
        }
    }

    struct FakeCustomers {
        exists: bool,
    }
    #[async_trait]
    impl CustomerRepository for FakeCustomers {
        async fn create(&self, _c: &Customer, _e: &OutboxMessage) -> DomainResult<()> {
            Ok(())
        }
        async fn list_in_tenant(
            &self,
            _t: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<Customer>> {
            Ok(vec![])
        }
        async fn find_in_tenant(&self, t: &TenantId, id: &Uuid) -> DomainResult<Option<Customer>> {
            Ok(self.exists.then(|| Customer {
                id: *id,
                tenant_id: t.clone(),
                name: "Acme".into(),
                email: None,
                phone: None,
                segment: None,
                created_at: Utc::now(),
            }))
        }
        async fn update(&self, _c: &Customer, _e: &OutboxMessage) -> DomainResult<()> {
            Ok(())
        }
    }

    struct StubClock;
    impl Clock for StubClock {
        fn now_utc(&self) -> DateTime<Utc> {
            Utc::now()
        }
    }

    fn service(customer_exists: bool) -> ActivityService {
        ActivityService::new(
            Arc::new(FakeActivities::default()),
            Arc::new(FakeCustomers {
                exists: customer_exists,
            }),
            Arc::new(StubClock),
        )
    }

    #[tokio::test]
    async fn log_parses_kind_and_inserts() {
        let svc = service(true);
        let a = svc
            .log(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "call".into(), // lower-case normalised
                "Intro call".into(),
                Some("went well".into()),
                None,
            )
            .await
            .unwrap();
        assert_eq!(a.kind, ActivityKind::Call);
        assert_eq!(a.subject, "Intro call");
    }

    #[tokio::test]
    async fn log_rejects_bad_kind_and_empty_subject() {
        let svc = service(true);
        assert!(matches!(
            svc.log(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "smoke_signal".into(),
                "x".into(),
                None,
                None
            )
            .await
            .unwrap_err(),
            DomainError::Validation(_)
        ));
        assert!(matches!(
            svc.log(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "CALL".into(),
                "  ".into(),
                None,
                None
            )
            .await
            .unwrap_err(),
            DomainError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn log_accepts_rfc3339_occurred_at_but_rejects_garbage() {
        let svc = service(true);
        let a = svc
            .log(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "MEETING".into(),
                "Demo".into(),
                None,
                Some("2026-06-11T09:30:00Z".into()),
            )
            .await
            .unwrap();
        assert_eq!(a.occurred_at.to_rfc3339(), "2026-06-11T09:30:00+00:00");
        assert!(matches!(
            svc.log(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "MEETING".into(),
                "Demo".into(),
                None,
                Some("yesterday".into())
            )
            .await
            .unwrap_err(),
            DomainError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn log_unknown_customer_is_not_found() {
        let svc = service(false);
        assert!(matches!(
            svc.log(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "TASK".into(),
                "Follow up".into(),
                None,
                None
            )
            .await
            .unwrap_err(),
            DomainError::NotFound(_)
        ));
    }
}
