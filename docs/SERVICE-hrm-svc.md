# SERVICE-hrm-svc.md

## 1. Mục tiêu & Domain

**Service**: `hrm-svc` [file:6][file:5]

**Domain chính**:

- HRM (Human Resource Management):
  - Nhân sự (Employee),
  - Chấm công (Attendance),
  - Nghỉ phép (Leave),
  - Tương lai: lương (Payroll). [file:6]

**Trách nhiệm**:

- Quản lý thông tin nhân sự & hiện diện. [file:6]
- Cung cấp dữ liệu HR cho reporting & AI (attendance patterns, headcount…). [file:7]

---

## 2. API chính (qua API Gateway)

Base path: `/api/v1/hrm`. [file:2]

- Employees:
  - `GET /api/v1/hrm/employees`
  - `POST /api/v1/hrm/employees`
  - `GET /api/v1/hrm/employees/{employee_id}`
  - `PATCH /api/v1/hrm/employees/{employee_id}` [file:2]

- Attendance:
  - `GET /api/v1/hrm/attendances`
  - `POST /api/v1/hrm/attendances` [file:2]

- Leaves:
  - `GET /api/v1/hrm/leaves`
  - `POST /api/v1/hrm/leaves`
  - `PATCH /api/v1/hrm/leaves/{leave_id}` [file:2]

---

## 3. Event publish/subscribe

Theo `ARCHITECTURE.md` & `EVENTS.md`: [file:6]

**Publish**

- `EmployeeHired`
- `EmployeeTerminated`
- `AttendanceRecorded` [file:6]

**Subscribe**

- `UserRegistered`, `TenantCreated` (auth-svc) – nếu cần sync user ↔ employee, cấu hình tenant. [file:6]

---

## 4. Data & Storage

- DB: PostgreSQL, schema: `hrm_svc` (hoặc `hrm`). [file:5][file:7]
- Bảng chính:
  - `employees`,
  - `attendances`,
  - `leaves`. [file:7]

---

## 5. Liên kết tài liệu

- `ARCHITECTURE.md` → mục 3.4 hrm-svc. [file:6]
- `AI-FIRST-ARCHITECTURE.md`. [file:5]
- `API-GATEWAY.md` → `/api/v1/hrm`. [file:2]
- `DATA-STRATEGY.md`. [file:7]
- `EVENTS.md` → HRM events.

---

## 6. Rule cho Dev & AI Agent

- Không tự làm auth trong HRM – luôn dùng auth-svc. [file:1][file:3]
- Luôn phát event cho hành động HR chính (hired/terminated/attendance). [file:6][file:7]