//! Leave use-cases (event-less; request then approve/reject workflow).

use std::sync::Arc;

use chrono::NaiveDate;
use uuid::Uuid;

use crate::domain::employees::ports::EmployeeRepository;
use crate::domain::leave::entities::{LeaveRequest, LeaveStatus};
use crate::domain::leave::ports::LeaveRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::types::{Clock, TenantId};

const DATE_FMT: &str = "%Y-%m-%d";

pub struct LeaveService {
    repo: Arc<dyn LeaveRepository>,
    employees: Arc<dyn EmployeeRepository>,
    clock: Arc<dyn Clock>,
}

impl LeaveService {
    pub fn new(
        repo: Arc<dyn LeaveRepository>,
        employees: Arc<dyn EmployeeRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repo,
            employees,
            clock,
        }
    }

    pub async fn request(
        &self,
        tenant_id: &TenantId,
        employee_id: Uuid,
        start_date: String,
        end_date: String,
        reason: Option<String>,
    ) -> DomainResult<LeaveRequest> {
        let start_date = parse_date(&start_date)?;
        let end_date = parse_date(&end_date)?;
        if end_date < start_date {
            return Err(DomainError::Validation(
                "end_date must be on or after start_date".into(),
            ));
        }
        self.employees
            .find_in_tenant(tenant_id, &employee_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("employee {employee_id}")))?;

        let request = LeaveRequest {
            id: Uuid::now_v7(),
            tenant_id: tenant_id.clone(),
            employee_id,
            start_date,
            end_date,
            reason: normalize(reason),
            status: LeaveStatus::Requested,
            created_at: self.clock.now_utc(),
        };
        self.repo.insert(&request).await?;
        Ok(request)
    }

    pub async fn list(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<LeaveRequest>> {
        self.repo.list_in_tenant(tenant_id, limit, offset).await
    }

    pub async fn get(&self, tenant_id: &TenantId, id: &Uuid) -> DomainResult<LeaveRequest> {
        self.repo
            .find_in_tenant(tenant_id, id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("leave request {id}")))
    }

    /// Approve or reject a pending request (REQUESTED → APPROVED | REJECTED).
    pub async fn decide(
        &self,
        tenant_id: &TenantId,
        id: &Uuid,
        new_status: LeaveStatus,
    ) -> DomainResult<LeaveRequest> {
        let mut request = self.get(tenant_id, id).await?;
        if !request.status.can_transition_to(new_status) {
            return Err(DomainError::Validation(format!(
                "cannot move leave request from {} to {}",
                request.status.as_str(),
                new_status.as_str()
            )));
        }
        request.status = new_status;
        self.repo.update_status(&request).await?;
        Ok(request)
    }
}

fn parse_date(raw: &str) -> DomainResult<NaiveDate> {
    NaiveDate::parse_from_str(raw.trim(), DATE_FMT)
        .map_err(|_| DomainError::Validation("date must be YYYY-MM-DD".into()))
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
    use chrono::{DateTime, Utc};
    use platform_outbox::OutboxMessage;

    use super::*;
    use crate::domain::employees::entities::{Employee, EmploymentStatus};

    #[derive(Default)]
    struct FakeLeave {
        items: Mutex<Vec<LeaveRequest>>,
    }
    #[async_trait]
    impl LeaveRepository for FakeLeave {
        async fn insert(&self, request: &LeaveRequest) -> DomainResult<()> {
            self.items.lock().unwrap().push(request.clone());
            Ok(())
        }
        async fn list_in_tenant(
            &self,
            _t: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<LeaveRequest>> {
            Ok(self.items.lock().unwrap().clone())
        }
        async fn find_in_tenant(
            &self,
            _t: &TenantId,
            id: &Uuid,
        ) -> DomainResult<Option<LeaveRequest>> {
            Ok(self
                .items
                .lock()
                .unwrap()
                .iter()
                .find(|r| r.id == *id)
                .cloned())
        }
        async fn update_status(&self, request: &LeaveRequest) -> DomainResult<()> {
            let mut items = self.items.lock().unwrap();
            if let Some(slot) = items.iter_mut().find(|r| r.id == request.id) {
                *slot = request.clone();
            }
            Ok(())
        }
    }

    struct FakeEmployees {
        exists: bool,
    }
    #[async_trait]
    impl EmployeeRepository for FakeEmployees {
        async fn create(&self, _e: &Employee, _ev: &OutboxMessage) -> DomainResult<()> {
            Ok(())
        }
        async fn list_in_tenant(
            &self,
            _t: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<Employee>> {
            Ok(vec![])
        }
        async fn find_in_tenant(&self, t: &TenantId, id: &Uuid) -> DomainResult<Option<Employee>> {
            Ok(self.exists.then(|| Employee {
                id: *id,
                tenant_id: t.clone(),
                full_name: "Jane".into(),
                position: "Eng".into(),
                email: None,
                status: EmploymentStatus::Active,
                created_at: Utc::now(),
            }))
        }
        async fn save_status(&self, _e: &Employee, _ev: &OutboxMessage) -> DomainResult<()> {
            Ok(())
        }
    }

    struct StubClock;
    impl Clock for StubClock {
        fn now_utc(&self) -> DateTime<Utc> {
            Utc::now()
        }
    }

    fn service(employee_exists: bool) -> LeaveService {
        LeaveService::new(
            Arc::new(FakeLeave::default()),
            Arc::new(FakeEmployees {
                exists: employee_exists,
            }),
            Arc::new(StubClock),
        )
    }

    #[tokio::test]
    async fn request_starts_pending() {
        let svc = service(true);
        let r = svc
            .request(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "2026-07-01".into(),
                "2026-07-05".into(),
                Some("holiday".into()),
            )
            .await
            .unwrap();
        assert_eq!(r.status, LeaveStatus::Requested);
    }

    #[tokio::test]
    async fn request_rejects_end_before_start_and_bad_date() {
        let svc = service(true);
        assert!(matches!(
            svc.request(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "2026-07-05".into(),
                "2026-07-01".into(),
                None
            )
            .await
            .unwrap_err(),
            DomainError::Validation(_)
        ));
        assert!(matches!(
            svc.request(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "07/01/2026".into(),
                "2026-07-05".into(),
                None
            )
            .await
            .unwrap_err(),
            DomainError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn request_unknown_employee_is_not_found() {
        let svc = service(false);
        assert!(matches!(
            svc.request(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "2026-07-01".into(),
                "2026-07-05".into(),
                None
            )
            .await
            .unwrap_err(),
            DomainError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn approve_then_rejects_second_decision() {
        let svc = service(true);
        let r = svc
            .request(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "2026-07-01".into(),
                "2026-07-05".into(),
                None,
            )
            .await
            .unwrap();
        let approved = svc
            .decide(&TenantId("t1".into()), &r.id, LeaveStatus::Approved)
            .await
            .unwrap();
        assert_eq!(approved.status, LeaveStatus::Approved);
        // Already decided: a second decision is rejected.
        assert!(matches!(
            svc.decide(&TenantId("t1".into()), &r.id, LeaveStatus::Rejected)
                .await
                .unwrap_err(),
            DomainError::Validation(_)
        ));
    }
}
