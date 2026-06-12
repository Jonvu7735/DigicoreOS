//! Attendance read model: one row per (employee, work_date), projected from the
//! `AttendanceRecorded` stream; backs the attendance half of the HRM summary
//! (`/reporting/hrm-summary`).

pub mod entities;
pub mod ports;
