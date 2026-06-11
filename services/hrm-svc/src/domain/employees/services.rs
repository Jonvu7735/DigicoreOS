//! Employee use-cases. Hiring and terminating emit HRM events.

use std::sync::Arc;

use event_models::hrm::{EmployeeHired, EmployeeTerminated, HrmEvent};
use event_models::EventHeader;
use uuid::Uuid;

use crate::domain::employees::entities::{Employee, EmploymentStatus};
use crate::domain::employees::ports::EmployeeRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::events::outbox_message;
use crate::domain::shared::types::{Clock, TenantId};

pub struct EmployeeService {
    repo: Arc<dyn EmployeeRepository>,
    clock: Arc<dyn Clock>,
}

impl EmployeeService {
    pub fn new(repo: Arc<dyn EmployeeRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { repo, clock }
    }

    pub async fn hire(
        &self,
        tenant_id: &TenantId,
        full_name: String,
        position: String,
        email: Option<String>,
    ) -> DomainResult<Employee> {
        let full_name = full_name.trim().to_string();
        let position = position.trim().to_string();
        if full_name.is_empty() {
            return Err(DomainError::Validation("full_name is required".into()));
        }
        if position.is_empty() {
            return Err(DomainError::Validation("position is required".into()));
        }

        let employee = Employee {
            id: Uuid::now_v7(),
            tenant_id: tenant_id.clone(),
            full_name,
            position,
            email: normalize(email),
            status: EmploymentStatus::Active,
            created_at: self.clock.now_utc(),
        };
        let event = outbox_message(&self.hired_event(&employee))?;
        self.repo.create(&employee, &event).await?;
        Ok(employee)
    }

    pub async fn list(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<Employee>> {
        self.repo.list_in_tenant(tenant_id, limit, offset).await
    }

    pub async fn get(&self, tenant_id: &TenantId, id: &Uuid) -> DomainResult<Employee> {
        self.repo
            .find_in_tenant(tenant_id, id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("employee {id}")))
    }

    pub async fn terminate(
        &self,
        tenant_id: &TenantId,
        id: &Uuid,
        reason: Option<String>,
    ) -> DomainResult<Employee> {
        let mut employee = self.get(tenant_id, id).await?;
        if employee.status != EmploymentStatus::Active {
            return Err(DomainError::Validation(
                "only an active employee can be terminated".into(),
            ));
        }
        employee.status = EmploymentStatus::Terminated;
        let event = outbox_message(&self.terminated_event(&employee, normalize(reason)))?;
        self.repo.save_status(&employee, &event).await?;
        Ok(employee)
    }

    fn hired_event(&self, e: &Employee) -> HrmEvent {
        HrmEvent::EmployeeHired(EmployeeHired {
            header: self.header(e, "EmployeeHired"),
            employee_id: e.id.to_string(),
            full_name: e.full_name.clone(),
            position: e.position.clone(),
        })
    }

    fn terminated_event(&self, e: &Employee, reason: Option<String>) -> HrmEvent {
        HrmEvent::EmployeeTerminated(EmployeeTerminated {
            header: self.header(e, "EmployeeTerminated"),
            employee_id: e.id.to_string(),
            reason,
        })
    }

    fn header(&self, e: &Employee, event_type: &str) -> EventHeader {
        EventHeader::new(
            Uuid::now_v7(),
            self.clock.now_utc(),
            e.tenant_id.0.clone(),
            "employee",
            e.id.to_string(),
            event_type,
            1,
        )
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
    use chrono::{DateTime, Utc};
    use platform_outbox::OutboxMessage;

    use super::*;

    #[derive(Default)]
    struct FakeRepo {
        items: Mutex<Vec<Employee>>,
        events: Mutex<Vec<String>>,
    }
    #[async_trait]
    impl EmployeeRepository for FakeRepo {
        async fn create(&self, employee: &Employee, event: &OutboxMessage) -> DomainResult<()> {
            self.items.lock().unwrap().push(employee.clone());
            self.events.lock().unwrap().push(event.event_type.clone());
            Ok(())
        }
        async fn list_in_tenant(
            &self,
            _t: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<Employee>> {
            Ok(self.items.lock().unwrap().clone())
        }
        async fn find_in_tenant(&self, _t: &TenantId, id: &Uuid) -> DomainResult<Option<Employee>> {
            Ok(self
                .items
                .lock()
                .unwrap()
                .iter()
                .find(|e| e.id == *id)
                .cloned())
        }
        async fn save_status(
            &self,
            employee: &Employee,
            event: &OutboxMessage,
        ) -> DomainResult<()> {
            let mut items = self.items.lock().unwrap();
            if let Some(slot) = items.iter_mut().find(|e| e.id == employee.id) {
                *slot = employee.clone();
            }
            self.events.lock().unwrap().push(event.event_type.clone());
            Ok(())
        }
    }

    struct StubClock;
    impl Clock for StubClock {
        fn now_utc(&self) -> DateTime<Utc> {
            Utc::now()
        }
    }

    fn service() -> (EmployeeService, Arc<FakeRepo>) {
        let repo = Arc::new(FakeRepo::default());
        (
            EmployeeService::new(repo.clone(), Arc::new(StubClock)),
            repo,
        )
    }

    #[tokio::test]
    async fn hire_emits_employee_hired_active() {
        let (svc, repo) = service();
        let e = svc
            .hire(
                &TenantId("t1".into()),
                "  Jane Doe  ".into(),
                "Engineer".into(),
                None,
            )
            .await
            .unwrap();
        assert_eq!(e.full_name, "Jane Doe");
        assert_eq!(e.status, EmploymentStatus::Active);
        assert_eq!(*repo.events.lock().unwrap(), vec!["EmployeeHired"]);
    }

    #[tokio::test]
    async fn hire_rejects_missing_fields() {
        let (svc, _) = service();
        assert!(matches!(
            svc.hire(&TenantId("t1".into()), " ".into(), "X".into(), None)
                .await
                .unwrap_err(),
            DomainError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn terminate_then_rejects_double_terminate() {
        let (svc, repo) = service();
        let e = svc
            .hire(&TenantId("t1".into()), "Jane".into(), "Eng".into(), None)
            .await
            .unwrap();
        let terminated = svc
            .terminate(&TenantId("t1".into()), &e.id, Some("layoff".into()))
            .await
            .unwrap();
        assert_eq!(terminated.status, EmploymentStatus::Terminated);
        assert_eq!(
            *repo.events.lock().unwrap(),
            vec!["EmployeeHired", "EmployeeTerminated"]
        );
        assert!(matches!(
            svc.terminate(&TenantId("t1".into()), &e.id, None)
                .await
                .unwrap_err(),
            DomainError::Validation(_)
        ));
    }
}
