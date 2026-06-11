//! Attendance use-cases. Recording attendance emits `AttendanceRecorded`.

use std::sync::Arc;

use chrono::{NaiveDate, NaiveTime};
use event_models::hrm::{AttendanceRecorded, HrmEvent};
use event_models::EventHeader;
use uuid::Uuid;

use crate::domain::attendance::entities::AttendanceRecord;
use crate::domain::attendance::ports::AttendanceRepository;
use crate::domain::employees::ports::EmployeeRepository;
use crate::domain::shared::error::{DomainError, DomainResult};
use crate::domain::shared::events::outbox_message;
use crate::domain::shared::types::{Clock, TenantId};

const DATE_FMT: &str = "%Y-%m-%d";
const TIME_FMT: &str = "%H:%M:%S";

pub struct AttendanceService {
    repo: Arc<dyn AttendanceRepository>,
    employees: Arc<dyn EmployeeRepository>,
    clock: Arc<dyn Clock>,
}

impl AttendanceService {
    pub fn new(
        repo: Arc<dyn AttendanceRepository>,
        employees: Arc<dyn EmployeeRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            repo,
            employees,
            clock,
        }
    }

    pub async fn record(
        &self,
        tenant_id: &TenantId,
        employee_id: Uuid,
        date: String,
        check_in: Option<String>,
        check_out: Option<String>,
    ) -> DomainResult<AttendanceRecord> {
        let date = NaiveDate::parse_from_str(date.trim(), DATE_FMT)
            .map_err(|_| DomainError::Validation("date must be YYYY-MM-DD".into()))?;
        let check_in = parse_time(check_in)?;
        let check_out = parse_time(check_out)?;
        if let (Some(ci), Some(co)) = (check_in, check_out) {
            if co < ci {
                return Err(DomainError::Validation(
                    "check_out must be at or after check_in".into(),
                ));
            }
        }
        self.employees
            .find_in_tenant(tenant_id, &employee_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("employee {employee_id}")))?;

        let record = AttendanceRecord {
            id: Uuid::now_v7(),
            tenant_id: tenant_id.clone(),
            employee_id,
            date,
            check_in,
            check_out,
            created_at: self.clock.now_utc(),
        };
        let event = outbox_message(&self.recorded_event(&record))?;
        self.repo.create(&record, &event).await?;
        Ok(record)
    }

    pub async fn list(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<AttendanceRecord>> {
        self.repo.list_in_tenant(tenant_id, limit, offset).await
    }

    pub async fn get(&self, tenant_id: &TenantId, id: &Uuid) -> DomainResult<AttendanceRecord> {
        self.repo
            .find_in_tenant(tenant_id, id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("attendance {id}")))
    }

    fn recorded_event(&self, r: &AttendanceRecord) -> HrmEvent {
        HrmEvent::AttendanceRecorded(AttendanceRecorded {
            header: EventHeader::new(
                Uuid::now_v7(),
                self.clock.now_utc(),
                r.tenant_id.0.clone(),
                "attendance",
                r.id.to_string(),
                "AttendanceRecorded",
                1,
            ),
            employee_id: r.employee_id.to_string(),
            date: r.date.format(DATE_FMT).to_string(),
            check_in: r.check_in.map(|t| t.format(TIME_FMT).to_string()),
            check_out: r.check_out.map(|t| t.format(TIME_FMT).to_string()),
        })
    }
}

/// Parse an optional `HH:MM:SS` time; blank becomes `None`, bad format errors.
fn parse_time(value: Option<String>) -> DomainResult<Option<NaiveTime>> {
    match value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
    {
        None => Ok(None),
        Some(s) => NaiveTime::parse_from_str(&s, TIME_FMT)
            .map(Some)
            .map_err(|_| DomainError::Validation("time must be HH:MM:SS".into())),
    }
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
    struct FakeAttendance {
        items: Mutex<Vec<AttendanceRecord>>,
        events: Mutex<Vec<String>>,
    }
    #[async_trait]
    impl AttendanceRepository for FakeAttendance {
        async fn create(
            &self,
            record: &AttendanceRecord,
            event: &OutboxMessage,
        ) -> DomainResult<()> {
            self.items.lock().unwrap().push(record.clone());
            self.events.lock().unwrap().push(event.event_type.clone());
            Ok(())
        }
        async fn list_in_tenant(
            &self,
            _t: &TenantId,
            _l: i64,
            _o: i64,
        ) -> DomainResult<Vec<AttendanceRecord>> {
            Ok(self.items.lock().unwrap().clone())
        }
        async fn find_in_tenant(
            &self,
            _t: &TenantId,
            id: &Uuid,
        ) -> DomainResult<Option<AttendanceRecord>> {
            Ok(self
                .items
                .lock()
                .unwrap()
                .iter()
                .find(|r| r.id == *id)
                .cloned())
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

    fn service(employee_exists: bool) -> (AttendanceService, Arc<FakeAttendance>) {
        let repo = Arc::new(FakeAttendance::default());
        let svc = AttendanceService::new(
            repo.clone(),
            Arc::new(FakeEmployees {
                exists: employee_exists,
            }),
            Arc::new(StubClock),
        );
        (svc, repo)
    }

    #[tokio::test]
    async fn record_emits_attendance_recorded() {
        let (svc, repo) = service(true);
        let r = svc
            .record(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "2026-06-11".into(),
                Some("09:00:00".into()),
                Some("17:30:00".into()),
            )
            .await
            .unwrap();
        assert_eq!(r.date.to_string(), "2026-06-11");
        assert_eq!(*repo.events.lock().unwrap(), vec!["AttendanceRecorded"]);
    }

    #[tokio::test]
    async fn record_rejects_bad_date_and_time() {
        let (svc, _) = service(true);
        assert!(matches!(
            svc.record(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "11/06/2026".into(),
                None,
                None
            )
            .await
            .unwrap_err(),
            DomainError::Validation(_)
        ));
        assert!(matches!(
            svc.record(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "2026-06-11".into(),
                Some("9am".into()),
                None
            )
            .await
            .unwrap_err(),
            DomainError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn record_unknown_employee_is_not_found() {
        let (svc, _) = service(false);
        assert!(matches!(
            svc.record(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "2026-06-11".into(),
                None,
                None
            )
            .await
            .unwrap_err(),
            DomainError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn record_rejects_checkout_before_checkin() {
        let (svc, _) = service(true);
        assert!(matches!(
            svc.record(
                &TenantId("t1".into()),
                Uuid::now_v7(),
                "2026-06-11".into(),
                Some("17:00:00".into()),
                Some("09:00:00".into())
            )
            .await
            .unwrap_err(),
            DomainError::Validation(_)
        ));
    }
}
