# SERVICE-reporting-svc.md

## 1. Mục tiêu & Domain

**Service**: `reporting-svc` [file:6][file:5]

**Domain chính**:

- Reporting / BI:
  - Fact/dimension,
  - Snapshot, aggregate,
  - Dashboard, báo cáo, export. [file:6][file:7]

**Trách nhiệm**:

- Xây data mart/reporting từ event & state. [file:7]
- Cung cấp API dashboard & báo cáo cho frontend & AI. [file:6]

---

## 2. API chính (qua API Gateway)

Base path: `/api/v1/reporting`. [file:2]

- Dashboard:
  - `GET /api/v1/reporting/overview`
  - `GET /api/v1/reporting/sales-summary`
  - `GET /api/v1/reporting/inventory-summary`
  - `GET /api/v1/reporting/crm-funnel`
  - `GET /api/v1/reporting/hrm-summary` [file:2][file:6]

- Reports chi tiết:
  - `GET /api/v1/reporting/orders`
  - `GET /api/v1/reporting/customers`
  - `GET /api/v1/reporting/employees` [file:2]

- Export:
  - `GET /api/v1/reporting/export` [file:2]

---

## 3. Event publish/subscribe

Theo `ARCHITECTURE.md` & `EVENTS.md`: [file:6]

**Publish**

- `ReportSnapshotCreated` [file:6]

**Subscribe**

- Tất cả các event chính:
  - `UserRegistered`, `TenantCreated` (auth-svc),
  - `OrderCreated`, `OrderPaid`, `StockAdjusted`, `InvoiceIssued` (core-erp-svc),
  - `CustomerCreated`, `CustomerUpdated`, `DealCreated`, `DealStageChanged` (crm-svc),
  - `EmployeeHired`, `EmployeeTerminated`, `AttendanceRecorded` (hrm-svc),
  - `AiInsightGenerated` (ai-svc). [file:6][file:7]

---

## 4. Data & Storage

- DB: PostgreSQL, schema: `reporting_svc`. [file:6][file:7]
- Bảng:
  - `fact_orders`, `fact_customers`, `fact_employees`, …
  - `dim_date`, `dim_product`, …
  - Snapshot/aggregate bảng riêng. [file:7]

Về sau: sync sang warehouse/lake theo `DATA-STRATEGY.md`. [file:7]

---

## 5. Liên kết tài liệu

- `ARCHITECTURE.md` → mục 3.5 reporting-svc. [file:6]
- `AI-FIRST-ARCHITECTURE.md`. [file:5]
- `API-GATEWAY.md` → `/api/v1/reporting`. [file:2]
- `DATA-STRATEGY.md`. [file:7]
- `EVENTS.md` → Reporting & related events.

---

## 6. Rule cho Dev & AI Agent

- Không viết logic nghiệp vụ ERP/CRM trong reporting:
  - Reporting chỉ **đọc** data từ event/state. [file:7]
- Luôn idempotent khi xử lý event (dựa trên `event_id`). [file:7]
- Mọi báo cáo mới:
  - Cân nhắc:
    - Fact/dimension nào cần,
    - Event/state nào là nguồn. [file:7]