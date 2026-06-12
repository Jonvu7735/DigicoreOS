//! Ports for the attendance read model (implemented in infra/db).

use async_trait::async_trait;

use crate::domain::attendance::entities::{AttendanceSummary, NewAttendanceFact};
use crate::domain::shared::error::DomainResult;
use crate::domain::shared::types::TenantId;

/// Write + read side of the attendance projection.
#[async_trait]
pub trait AttendanceProjection: Send + Sync {
    /// Project one `AttendanceRecorded`, **idempotently** on
    /// (tenant, employee, work_date); a same-day follow-up merges via COALESCE.
    async fn apply_attendance_recorded(&self, rec: &NewAttendanceFact) -> DomainResult<()>;
    /// Attendance rollup (record count + distinct employees) for a tenant.
    async fn summary(&self, tenant: &TenantId) -> DomainResult<AttendanceSummary>;
}
