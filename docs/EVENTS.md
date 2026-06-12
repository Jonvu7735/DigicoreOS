# EVENTS.md

## 1. Mục tiêu & Phạm vi

### 1.1. Mục tiêu

Tài liệu này chuẩn hoá **event nghiệp vụ (business events)** trong nền tảng Rust SaaS AI-First, nhằm: [file:6][file:5][file:7]

- Định nghĩa **danh sách event chuẩn** (OrderCreated, UserRegistered, AiInsightGenerated, …).
- Chuẩn hoá **schema (field, kiểu dữ liệu, ý nghĩa)** cho từng event.
- Chuẩn hoá **subject/topic** trên event bus (NATS/Kafka).
- Xác định **service nào publish / service nào subscribe** cho mỗi event.

Từ đó:

- Giảm trùng lặp, tránh lệch schema giữa các service.
- Giúp AI Agent có **một nguồn sự thật duy nhất** khi sinh code event-models & EventPublisher. [file:5]

### 1.2. Phạm vi

Tài liệu này:

- Tập trung vào **business event**:
  - Sự kiện nghiệp vụ như `OrderCreated`, `CustomerUpdated`, `EmployeeHired`, …
- Không mô tả:
  - Log kỹ thuật (error log, debug log) → xem `OBSERVABILITY.md`. [file:4]
  - Chiến lược lưu trữ dữ liệu (event store, TSDB, warehouse) → xem `DATA-STRATEGY.md`. [file:7]
  - Flow business chi tiết → xem `ARCHITECTURE.md`. [file:6]
  - Layout code chi tiết → xem `AI-FIRST-ARCHITECTURE.md`. [file:5]

---

## 2. Nguyên tắc thiết kế event

### 2.1. Event là gì?

- **Business event**:
  - Là sự kiện nghiệp vụ quan trọng đã xảy ra:
    - `OrderCreated`, `OrderPaid`, `StockAdjusted`, `UserRegistered`, …
  - Không phải log kỹ thuật thuần tuý. [file:6][file:7][file:4]

### 2.2. Nguyên tắc tổng quát

- Event là **append-only**:
  - Không sửa, không xoá event đã phát/ghi. [file:7]
- Mỗi event:
  - Có `event_id` duy nhất,
  - Có `occurred_at` (thời điểm xảy ra),
  - Gắn với `tenant_id` (multi-tenant),
  - Gắn với `aggregate_type`, `aggregate_id`, `version` nếu cần. [file:7][file:5]

- Khi logic thay đổi:
  - Không sửa event cũ,
  - Tạo event mới (có thể version mới, hoặc thêm field new, giữ backward-compatible). [file:7]

### 2.3. Naming convention

- Tên event (type) dùng **PascalCase**, kết thúc bằng động từ ở quá khứ:
  - `OrderCreated`, `OrderPaid`, `OrderCancelled`,
  - `CustomerCreated`, `CustomerUpdated`,
  - `EmployeeHired`, `EmployeeTerminated`,
  - `AiInsightGenerated`. [file:6][file:5]

- Subject trên NATS/Kafka:
  - Dạng: `platform.<domain>.<entity>.<action_pasttense>`
  - Ví dụ:
    - `platform.erp.order.created`
    - `platform.erp.order.paid`
    - `platform.crm.customer.created`
    - `platform.hrm.employee.hired`
    - `platform.ai.insight.generated` [file:5]

### 2.4. Schema chung cho mọi event

Tất cả event đều tuân thủ schema “phần header” chung:

```text
event_id       UUID          # ID duy nhất của event
occurred_at    TIMESTAMPTZ   # Thời điểm event xảy ra (UTC)
tenant_id      TEXT          # Tenant liên quan
aggregate_type TEXT          # Loại aggregate (order, user, customer, employee, ...)
aggregate_id   TEXT          # ID aggregate
event_type     TEXT          # Tên event (OrderCreated, UserRegistered, ...)
version        INT           # Version của event (1, 2, ...)
```

Phần payload chi tiết sẽ tuỳ theo từng event.

Trong code Rust (`event-models` crate), có thể biểu diễn:

```rust
pub struct EventHeader {
    pub event_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub tenant_id: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub version: i32,
}
```

---

## 3. Danh sách event theo domain

### 3.1. Auth & Tenant Events (auth-svc)

#### 3.1.1. UserRegistered

- **Subject**: `platform.auth.user.registered`
- **Producer**: `auth-svc`
- **Consumers**:
  - `crm-svc` (nếu cần gắn user → owner khách hàng),
  - `hrm-svc` (nếu sync user ↔ employee),
  - `reporting-svc` (audit & reporting). [file:6][file:7]

**Payload (Rust struct gợi ý)**

```rust
pub struct UserRegistered {
    pub header: EventHeader,
    pub user_id: String,
    pub email: String,
    pub display_name: String,
    pub is_active: bool,
}
```

#### 3.1.2. UserUpdated

- **Subject**: `platform.auth.user.updated`
- **Producer**: `auth-svc`
- **Consumers**:
  - `crm-svc`, `hrm-svc`, `reporting-svc`. [file:6]

**Payload**

```rust
pub struct UserUpdated {
    pub header: EventHeader,
    pub user_id: String,
    pub email: String,
    pub display_name: String,
    pub is_active: bool,
}
```

#### 3.1.3. TenantCreated

- **Subject**: `platform.auth.tenant.created`
- **Producer**: `auth-svc`
- **Consumers**:
  - `hrm-svc` (tạo cấu trúc HRM mặc định),
  - `core-erp-svc`, `crm-svc`, `reporting-svc`, `ai-svc` (khởi tạo dữ liệu mặc định). [file:6][file:7]

**Payload**

```rust
pub struct TenantCreated {
    pub header: EventHeader,
    pub tenant_id: String,
    pub tenant_name: String,
    pub plan: String,
}
```

---

### 3.2. ERP Events (core-erp-svc)

#### 3.2.1. OrderCreated

- **Subject**: `platform.erp.order.created`
- **Producer**: `core-erp-svc`
- **Consumers**:
  - `reporting-svc` (fact_orders),
  - `crm-svc` (timeline khách hàng),
  - `ai-svc` (feature/insight). [file:6][file:7]

**Payload**

```rust
pub struct OrderCreated {
    pub header: EventHeader,
    pub order_id: String,
    pub customer_id: String,
    pub total_amount: i64,
    pub currency: String,
    pub status: String, // e.g. "NEW"
}
```

#### 3.2.2. OrderStatusChanged

- **Subject**: `platform.erp.order.status_changed`
- **Producer**: `core-erp-svc`
- **Consumers**:
  - `reporting-svc`,
  - `crm-svc`,
  - `ai-svc`. [file:6]

**Payload**

```rust
pub struct OrderStatusChanged {
    pub header: EventHeader,
    pub order_id: String,
    pub old_status: String,
    pub new_status: String,
}
```

#### 3.2.3. OrderPaid

- **Subject**: `platform.erp.order.paid`
- **Producer**: `core-erp-svc`
- **Consumers**:
  - `reporting-svc` (doanh thu),
  - `crm-svc` (lifecycle khách hàng),
  - `ai-svc` (CLV, scoring). [file:6][file:7]

**Payload**

```rust
pub struct OrderPaid {
    pub header: EventHeader,
    pub order_id: String,
    pub amount_paid: i64,
    pub payment_method: String,
}
```

#### 3.2.4. StockAdjusted

- **Subject**: `platform.erp.inventory.stock_adjusted`
- **Producer**: `core-erp-svc`
- **Consumers**:
  - `reporting-svc` (snapshot tồn kho),
  - `ai-svc` (anomaly detection). [file:6][file:7]

**Payload**

```rust
pub struct StockAdjusted {
    pub header: EventHeader,
    pub product_id: String,
    pub warehouse_id: String,
    pub delta: i64,       // số lượng tăng/giảm
    pub reason: String,   // e.g. "order", "manual_adjustment"
}
```

#### 3.2.5. InvoiceIssued

- **Subject**: `platform.erp.invoice.issued`
- **Producer**: `core-erp-svc`
- **Consumers**:
  - `reporting-svc`,
  - `ai-svc` (phân tích dòng tiền). [file:6][file:7]

**Payload (gợi ý)**

```rust
pub struct InvoiceIssued {
    pub header: EventHeader,
    pub invoice_id: String,
    pub order_id: String,
    pub amount: i64,
    pub currency: String,
}
```

---

### 3.3. CRM Events (crm-svc)

#### 3.3.1. CustomerCreated

- **Subject**: `platform.crm.customer.created`
- **Producer**: `crm-svc`
- **Consumers**:
  - `core-erp-svc` (optional sync),
  - `reporting-svc`,
  - `ai-svc`. [file:6][file:7]

```rust
pub struct CustomerCreated {
    pub header: EventHeader,
    pub customer_id: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub segment: Option<String>,
}
```

#### 3.3.2. CustomerUpdated

- **Subject**: `platform.crm.customer.updated`
- **Producer**: `crm-svc`
- **Consumers**: tương tự `CustomerCreated`. [file:6]

```rust
pub struct CustomerUpdated {
    pub header: EventHeader,
    pub customer_id: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub segment: Option<String>,
}
```

#### 3.3.3. DealCreated

- **Subject**: `platform.crm.deal.created`
- **Producer**: `crm-svc`
- **Consumers**:
  - `reporting-svc` (funnel, pipeline),
  - `ai-svc` (lead scoring). [file:6][file:7]

```rust
pub struct DealCreated {
    pub header: EventHeader,
    pub deal_id: String,
    pub customer_id: String,
    pub amount_estimate: i64,
    pub stage: String,
}
```

#### 3.3.4. DealStageChanged

- **Subject**: `platform.crm.deal.stage_changed`
- **Producer**: `crm-svc`
- **Consumers**: `reporting-svc`, `ai-svc`. [file:6]

```rust
pub struct DealStageChanged {
    pub header: EventHeader,
    pub deal_id: String,
    pub old_stage: String,
    pub new_stage: String,
}
```

---

### 3.4. HRM Events (hrm-svc)

#### 3.4.1. EmployeeHired

- **Subject**: `platform.hrm.employee.hired`
- **Producer**: `hrm-svc`
- **Consumers**:
  - `auth-svc` (tự tạo user, nếu policy cho phép),
  - `reporting-svc`,
  - `ai-svc`. [file:6][file:7]

```rust
pub struct EmployeeHired {
    pub header: EventHeader,
    pub employee_id: String,
    pub full_name: String,
    pub position: String,
}
```

#### 3.4.2. EmployeeTerminated

- **Subject**: `platform.hrm.employee.terminated`
- **Producer**: `hrm-svc`
- **Consumers**:
  - `auth-svc` (vô hiệu user),
  - `reporting-svc`,
  - `ai-svc`. [file:6]

```rust
pub struct EmployeeTerminated {
    pub header: EventHeader,
    pub employee_id: String,
    pub reason: Option<String>,
}
```

#### 3.4.3. AttendanceRecorded

- **Subject**: `platform.hrm.attendance.recorded`
- **Producer**: `hrm-svc`
- **Consumers**:
  - `reporting-svc` (attendance report),
  - `ai-svc` (pattern vắng mặt bất thường). [file:6][file:7]

```rust
pub struct AttendanceRecorded {
    pub header: EventHeader,
    pub employee_id: String,
    pub date: String,   // YYYY-MM-DD
    pub check_in: Option<String>,  // HH:MM:SS
    pub check_out: Option<String>, // HH:MM:SS
}
```

---

### 3.5. Reporting Events (reporting-svc)

#### 3.5.1. ReportSnapshotCreated

- **Subject**: `platform.reporting.snapshot.created`
- **Producer**: `reporting-svc`
- **Consumers**:
  - `ai-svc` (lấy snapshot làm input analysis advanced),
  - Cơ chế khác nếu cần. [file:6][file:7]

```rust
pub struct ReportSnapshotCreated {
    pub header: EventHeader,
    pub snapshot_id: String,
    pub snapshot_type: String, // e.g. "inventory", "sales"
}
```

---

### 3.6. AI Events (ai-svc)

#### 3.6.1. AiInsightGenerated

- **Subject**: `platform.ai.insight.generated`
- **Producer**: `ai-svc`
- **Consumers**:
  - `reporting-svc` (hiển thị insight trên dashboard),
  - `notification-svc` (nếu có, gửi cảnh báo). [file:6][file:7]

```rust
pub struct AiInsightGenerated {
    pub header: EventHeader,
    pub insight_id: String,
    pub category: String,   // e.g. "sales_anomaly", "churn_risk"
    pub summary: String,    // tóm tắt insight
}
```

---

### 3.7. Trade-Export Events (trade-export-svc — vertical)

> Vertical module (`verticals/trade-export-svc`). Its event payloads are defined
> IN the vertical, **not** in the shared `event-models` crate (which stays
> core/platform-only). The vertical *consumes* core events but *owns* the events
> it publishes.

#### 3.7.1. ShipmentBooked

- **Subject**: `platform.trade_export.shipment.booked`
- **Producer**: `trade-export-svc`
- **Consumes**: `platform.erp.order.paid` (OrderPaid) → drafts an export shipment.
- **Consumers**: bất kỳ service logistics/xuất khẩu nào quan tâm (hiện chưa có).

```rust
// Defined in verticals/trade-export-svc (reuses the shared EventHeader envelope).
pub struct ShipmentBooked {
    pub header: EventHeader,
    pub shipment_id: String,
    pub reference: String,
    pub destination_country: String, // ISO-3166 alpha-2
    pub order_id: Option<String>,    // ERP order this shipment fulfils, if any
}
```

---

## 4. Schema Rust & Crate event-models

### 4.1. Cấu trúc crate event-models

Như đã mô tả ở `AI-FIRST-ARCHITECTURE.md`, crate `event-models` nên có cấu trúc: [file:5]

```text
event-models/
  src/
    lib.rs
    header.rs
    auth_events.rs
    erp_events.rs
    crm_events.rs
    hrm_events.rs
    reporting_events.rs
    ai_events.rs
```

Trong đó:

- `header.rs` định nghĩa `EventHeader`.
- Mỗi file domain định nghĩa các event struct tương ứng.

### 4.2. Quy tắc versioning

- Trường `version` trong `EventHeader`:
  - Dùng để phân biệt phiên bản schema của cùng một event type. [file:7]
- Khi thêm field **không phá vỡ** (backward-compatible):
  - Tăng version nếu cần, nhưng consumer cũ có thể ignore field mới. [file:7]
- Khi thay đổi phá vỡ (breaking):
  - Có thể tạo **event type mới** (vd: `OrderCreatedV2`) hoặc quản lý version cẩn thận ở consumer. [file:7][file:5]

---

## 5. Rule cho Dev & AI Agent

### 5.1. Khi thêm event mới

- Dev/AI phải:
  - Đặt tên event theo naming convention (PascalCase, động từ quá khứ). [file:5]
  - Chọn subject theo pattern `platform.<domain>.<entity>.<action>`. [file:5]
  - Định nghĩa schema:
    - Update `EVENTS.md`,
    - Thêm struct tương ứng trong `event-models`. [file:5]
  - Cập nhật:
    - Producer (publish ở service),
    - Consumer (xử lý event ở service khác). [file:6][file:7]

### 5.2. AI Agent

- **AI Agent phải:**
  - Dùng `EVENTS.md` làm **nguồn chuẩn** cho event:
    - Tên event,
    - Field, kiểu dữ liệu,
    - Subject, producer, consumer. [file:5][file:6][file:7]
- **AI Agent được:**
  - Sinh:
    - Struct event trong crate `event-models`,
    - Implement `EventPublisher`/consumer cho NATS/Kafka,
    - Test idempotent xử lý event. [file:5]
- **AI Agent không được:**
  - Tự đổi schema event (thêm/bớt field) mà không update doc & crate `event-models`.
  - Tự đổi subject trên bus (NATS/Kafka) nếu không có yêu cầu rõ và không sửa toàn bộ nơi sử dụng.

---

## 6. Tóm tắt

- `EVENTS.md` + `event-models` crate là **xương sống** của kiến trúc event-driven trong nền tảng Rust SaaS AI-First. [file:6][file:5]
- Các file khác:
  - `ARCHITECTURE.md` dùng event này để mô tả flow,
  - `DATA-STRATEGY.md` dùng làm nguồn cho lớp Event/Analytics,
  - `OBSERVABILITY.md` mô tả cách kết hợp event với log/metrics/traces,
  - AI Agent dựa vào đây để sinh/hiểu code event. [file:6][file:7][file:4][file:5]