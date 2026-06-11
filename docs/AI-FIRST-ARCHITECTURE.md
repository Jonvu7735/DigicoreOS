# AI-FIRST-ARCHITECTURE.md

> Mục tiêu: Thiết kế codebase và hệ thống sao cho **AI Agent có thể đọc, hiểu, sinh code, refactor, và test** một cách an toàn và nhất quán – không chỉ “xài AI như Copilot”. [file:5]

---

## 1. Mục tiêu & Phạm vi

### 1.1. Mục tiêu

- Chuẩn hoá **kiến trúc code** cho toàn bộ service Rust của nền tảng SaaS đa sản phẩm (ERP, CRM, HRM, Reporting/BI, AI-svc, các app dọc). [file:5]
- Đảm bảo code:
  - Dễ đọc, dễ review với con người.
  - Dễ hiểu, dễ sinh code, dễ refactor với AI Agent. [file:5]
- Hỗ trợ triết lý **AI-First**:
  - Kiến trúc, boundary, schema được thiết kế từ đầu để AI có thể:
    - Sinh boilerplate (handler, DTO, adapter, migration),
    - Sinh test, refactor lặp lại,
    - Phân tích log/metrics sau này. [file:5]

### 1.2. Phạm vi

- Tập trung vào:
  - Cấu trúc thư mục (layout) chuẩn cho mỗi service Rust. [file:5]
  - Nguyên tắc tách domain/infra/api/bootstrap. [file:5]
  - Contracts-first (API, event, DB). [file:5]
  - Observability-first (log/metrics/traces) ở mức code. [file:5][file:4]
  - Quy trình sử dụng AI Agent trong repo. [file:5]
- Không mô tả:
  - Luồng business end-to-end, data flow nghiệp vụ  
    → xem `ARCHITECTURE.md`. [file:6]
  - Chiến lược dữ liệu 4 lớp, 6 nhóm  
    → xem `DATA-STRATEGY.md`. [file:7]
  - Danh sách API endpoint  
    → xem `API-GATEWAY.md`. [file:2]

---

## 2. Nguyên tắc kiến trúc AI-First

### 2.1. Modular & Service-Oriented

- Nền tảng gồm nhiều service nhỏ: [file:5][file:6]
  - `auth-svc`: Auth, user, tenant, roles & permissions.
  - `core-erp-svc`: Order, inventory, purchasing, finance (ERP lõi).
  - `crm-svc`: lead, opportunity, pipeline, campaign.
  - `hrm-svc`: hồ sơ nhân sự, chấm công, tính lương.
  - `reporting-svc`: aggregate, dashboard, export.
  - `ai-svc`: đề xuất, dự báo, scoring, anomaly detection.
  - `notification-svc` (tương lai): email, SMS, push, in-app.
- Mỗi service quản lý **một hoặc vài bounded context** nghiệp vụ rõ ràng. [file:5]

### 2.2. Separation of Concerns (tách biệt mối quan tâm)

- Mỗi service tuân theo pattern: [file:5]

  - `domain`: nghiệp vụ thuần, **không phụ thuộc** HTTP/DB/messaging.
  - `infra`: IO, DB, messaging, security, external integration.
  - `api`: HTTP layer (Axum), DTO request/response.
  - `bootstrap`: config, wiring (DI – Dependency Injection: tiêm phụ thuộc).

- Domain **không import**:
  - Axum,
  - SQLx/SeaORM,
  - NATS/Kafka,
  - logging, config, v.v. [file:5]

### 2.3. Contracts First

- Mọi giao tiếp được xây trên **hợp đồng rõ ràng**: [file:5]

  - HTTP:
    - DTO request/response (struct) nằm trong `api/http/dto`.
    - Spec tổng thể trong `API-GATEWAY.md` + OpenAPI. [file:2][file:5]
  - Event:
    - Struct event trong crate chung `event-models`. [file:5]
    - Mọi service dùng cùng crate để tránh lệch schema.
  - DB:
    - Mỗi service có schema riêng, migration riêng. [file:5][file:7]
    - Không query chéo schema (cross-service) trực tiếp.

### 2.4. Data & Observability là First-Class

- Logging JSON structured (ít nhất: `timestamp`, `service`, `tenant_id`, `trace_id`, `level`, `message`). [file:5][file:4]
- Tracing với trace/span ID cho mọi request và event. [file:5][file:4]
- Metrics chuẩn cho mỗi service:
  - latency, error rate, throughput,
  - business metrics (orders_created, failed_payments, crm_deals_won, …). [file:5][file:4]
- Đây là nền cho:
  - Monitoring & alerting,
  - AI Ops (AI phân tích hành vi hệ thống). [file:4][file:7]

### 2.5. AI-Friendly Codebase

- Layout, naming, pattern **nhất quán giữa các service**: [file:5]
- Documentation bắt buộc cho:
  - Module,
  - Contract (DTO, event, port). [file:5]
- Hạn chế “magic”:
  - Ưu tiên code tường minh (explicit config, explicit error handling). [file:5]

---

## 3. Kiến trúc tổng thể (ở góc nhìn code)

### 3.1. Danh sách service & nhiệm vụ (code view)

- `auth-svc`
  - Auth, user, tenant, roles, permissions.
  - Cấp/verify JWT, model RBAC trong DB. [file:5][file:1]
- `core-erp-svc`
  - Order, inventory, invoice, payment, finance. [file:5]
- `crm-svc`
  - Customer, deal, activity, pipeline. [file:5]
- `hrm-svc`
  - Employee, attendance, leave, payroll. [file:5]
- `reporting-svc`
  - ETL từ event/state → bảng reporting, API dashboard & export. [file:5][file:7]
- `ai-svc`
  - Kết nối LLM, vector DB, sinh insight, scoring. [file:5][file:7]

### 3.2. Giao tiếp giữa các service (code-level)

- HTTP synchronous:
  - Handler Axum → service domain → repo → DB/event. [file:5][file:6]
- Event asynchronous:
  - Domain gọi port `EventPublisher`.
  - `infra/messaging` implement port bằng NATS/Kafka. [file:5][file:6][file:7]

---

## 4. Layout chuẩn cho mỗi service Rust

Cấu trúc thư mục chuẩn: [file:5]

```text
<service-name>/
  Cargo.toml
  AI-FIRST-ARCHITECTURE.md   # Bản rút gọn, link về file gốc ở root

  src/
    main.rs

    bootstrap/
      config.rs              # Load config từ env
      wiring.rs              # Khởi tạo AppState, router, DI

    api/
      mod.rs
      http/
        routes.rs            # Định tuyến Axum
        handlers/            # Handlers theo domain
        dto/                 # DTO request/response (serde)
        middleware/          # Auth, logging, trace...

    domain/
      mod.rs
      shared/
        error.rs             # DomainError, DomainResult
        types.rs             # UserId, TenantId, Money, ...
      <bounded-context>/
        entities.rs          # Entities thuần domain
        ports.rs             # Traits (Repo, EventPublisher, ...)
        services.rs          # Usecase/service layer

    infra/
      mod.rs

      db/
        postgres.rs          # Pool, migration
        <x>_repo_pg.rs       # Impl Repo trait (UserRepo, OrderRepo, ...)

      messaging/
        nats.rs              # Impl EventPublisher cho NATS

      security/
        jwt.rs               # JWT
        password.rs          # Hash/verify password

      time/
        clock.rs             # Clock trait

    utils/
      logging.rs             # Tracing setup
      id.rs                  # ID generator
```

**Nguyên tắc dependency**: [file:5]

- `domain` không phụ thuộc `api`, `infra`, `utils`.
- `infra` chỉ implement trait được định nghĩa ở `domain`.
- `api` chỉ gọi domain qua service/trait, không truy cập DB/message broker trực tiếp.

---

## 5. Chiến lược Database per service

### 5.1. Database-per-service trên shared instance

- Sử dụng 1 PostgreSQL cluster (Cloud SQL). [file:5][file:7]
- Mỗi service có schema riêng:

| Service         | Schema/DB gợi ý   |
|-----------------|-------------------|
| `auth-svc`      | `auth_svc`        |
| `core-erp-svc`  | `erp_core_svc`    |
| `crm-svc`       | `crm_svc`         |
| `hrm-svc`       | `hrm_svc`         |
| `reporting-svc` | `reporting_svc`   |

**Quy ước**: [file:5][file:7]

- Mỗi service chỉ kết nối tới schema/DB của mình (enforced bằng config + wiring).
- Không query trực tiếp sang schema của service khác.
- Cross-service data:
  - Qua HTTP API (đọc theo nhu cầu),
  - Qua event (async) cho reporting, denormalization.

### 5.2. Migration & ORM

- ORM/driver:
  - SeaORM hoặc SQLx (theo chọn của từng service). [file:5]
- Mapping:
  - Entity/Model ORM chỉ tồn tại ở `infra/db`.
  - Domain entity không phụ thuộc ORM (mapping domain ↔ ORM tách biệt). [file:5]
- Migration:
  - Sử dụng SeaORM CLI hoặc SQLx migrate.
  - Migration script versioned trong repo từng service. [file:5]
- Không implement logic domain phức tạp bằng DB trigger/procedure. [file:5]

---

## 6. Event-Driven Architecture (NATS trước, Kafka/Pub/Sub sau)

### 6.1. Crate dùng chung cho event

- Crate: `event-models/`. [file:5]

```text
event-models/
  src/
    lib.rs
    order_events.rs
    inventory_events.rs
    user_events.rs
    crm_events.rs
    hrm_events.rs
    ai_events.rs
```

Ví dụ: [file:5]

```rust
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderCreated {
    pub event_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub tenant_id: String,
    pub order_id: String,
    pub total_amount: i64,
}
```

Tất cả service dùng cùng crate này → tránh lệch schema, AI Agent chỉ cần follow **một nguồn sự thật** cho event. [file:5]

### 6.2. Ports & adapter cho event

Trong domain (vd `domain/shared/ports.rs`): [file:5]

```rust
#[async_trait::async_trait]
pub trait EventPublisher {
    async fn publish_order_created(&self, event: OrderCreated) -> DomainResult<()>;
    // các event khác nếu cần
}
```

Trong `infra/messaging/nats.rs`:

- Implement `EventPublisher` cho NATS. [file:5]
- Quy ước subject:

  - `platform.order.created`
  - `platform.order.paid`
  - `platform.inventory.stock_adjusted`
  - `platform.user.created`
  - `platform.crm.deal_created`
  - `platform.hrm.employee_hired`
  - `platform.ai.insight_generated`
  - …

### 6.3. Quy tắc sử dụng event

- Service publish event **sau khi** commit transaction (outbox pattern). [file:5][file:7]
- Service nhận event (reporting-svc, ai-svc, …) phải:
  - Xử lý idempotent (dựa trên `event_id` hoặc event store riêng).
  - Có cơ chế DLQ (Dead Letter Queue – hàng đợi lỗi) hoặc log lỗi, không crash hệ thống chính. [file:5][file:7]
- Event là business event (sự kiện nghiệp vụ), **không phải** log kỹ thuật thuần tuý. [file:5][file:7]

---

## 7. Quy tắc viết API & DTO (AI-Friendly)

### 7.1. Tên route & HTTP method rõ ràng

- Ví dụ: [file:5][file:2]
  - `POST /api/v1/erp/orders` – tạo đơn.
  - `GET /api/v1/erp/orders/{id}` – chi tiết đơn.
  - `GET /api/v1/erp/orders?tenant_id=&status=` – danh sách.
- Không encode logic phức tạp vào URL mơ hồ.

### 7.2. DTO tách khỏi entity

- DTO:
  - Trong `api/http/dto/*`. [file:5]
- Domain entity:
  - Trong `domain/<bounded-context>/entities.rs`. [file:5]
- Không dùng entity domain làm response trực tiếp → tránh coupling domain ↔ API. [file:5]

### 7.3. Chuẩn hoá error response

- Domain error: `DomainError` (`NotFound`, `Validation`, `PermissionDenied`, `Conflict`, `Internal`, …). [file:5]
- HTTP error: `ApiError` map từ `DomainError` sang:
  - HTTP status code (404, 400, 403, 409, 500, …),
  - JSON body chuẩn, ví dụ: [file:5]

```json
{
  "error_code": "ORDER_NOT_FOUND",
  "message": "Order not found",
  "details": null
}
```

---

## 8. Observability & Logging trong code

### 8.1. Logging

- Sử dụng `tracing` + `tracing-subscriber`. [file:5][file:4]
- Output JSON với tối thiểu các field:

  - `timestamp`
  - `service`
  - `env`
  - `level`
  - `tenant_id` (nếu có)
  - `user_id` (nếu có)
  - `trace_id`, `span_id`
  - `message`

### 8.2. Tracing

- Mỗi request HTTP:
  - Tạo root span. [file:5][file:4]
- Mỗi event publish/consume:
  - Nếu có `trace_id` từ upstream → propagate.
  - Nếu không → có thể tạo trace riêng. [file:5][file:4]

### 8.3. Metrics

- Cho mọi service: [file:5][file:4]
  - `http_requests_total{service, route, method, status}`
  - `http_request_duration_seconds_bucket{service, route, method}`
  - `events_published_total{service, event_type}`
  - `events_consumed_total{service, event_type}`
- Về sau có thể thêm:
  - Metric theo tenant, feature, loại sản phẩm (ERP/CRM/HRM).

---

## 9. Quy ước & Quy trình sử dụng AI Agent

### 9.1. Việc AI được khuyến khích làm

- Sinh: [file:5]
  - Handler Axum mới theo pattern có sẵn.
  - DTO request/response từ spec (issue/docs).
  - Implementation chi tiết cho service/port đã định nghĩa trong domain.
  - Test unit/integration cho domain service (dùng fake/mocked repo).
  - Migration DB khi schema đã được mô tả rõ.
- Refactor:
  - Tách function/module nếu code trùng lặp.
  - Đổi tên cho rõ nghĩa sau khi con người chốt naming. [file:5]

### 9.2. Việc con người phải quyết

- Boundary domain & service (khi thêm hoặc tách service). [file:5][file:6]
- Schema DB mới, event mới (field & semantics). [file:5][file:7]
- Thay đổi contract API breaking change. [file:5][file:2]
- Chiến lược bảo mật, multi-tenant, phân quyền, compliance. [file:5][file:1][file:3]

### 9.3. Checklist khi dùng AI trong repo

- Trước khi nhờ AI:
  - Cần có mô tả rõ ràng (issue/comment/docs) về:
    - Input, output mong muốn,
    - File/module liên quan,
    - Constraint (performance, security, backward compatibility). [file:5]
- Sau khi AI sinh code:
  - Luôn:
    - Review logic,
    - Chạy test hoặc yêu cầu AI sinh test rồi chạy,
    - Đảm bảo style/naming khớp guideline này. [file:5]

---

## 10. Tài liệu bổ sung & Liên kết

Trong **root repo**: [file:5]

- `ARCHITECTURE.md`:
  - Sơ đồ hệ thống: service, data flow, boundary giữa sản phẩm ERP/CRM/HRM/BI. [file:6]
- `API-GATEWAY.md`:
  - Bề mặt API: route, method, versioning, multi-tenant. [file:2]
- `EVENTS.md`:
  - Danh sách event, schema, producer/consumer, topic/subject. [file:5][file:6]
- `DATA-STRATEGY.md`:
  - Chiến lược DB, backup, retention, multi-tenant, phân vùng dữ liệu. [file:7]
- `AI-FIRST-ARCHITECTURE.md`:
  - File guideline này.

Trong **mỗi service**:

- `SERVICE-README.md`:
  - Mô tả chức năng, route chính, bounded context. [file:5]
- `AI-FIRST-ARCHITECTURE.md`:
  - Bản rút gọn (link về file root, có thể thêm chi tiết riêng cho service). [file:5]

---

## 11. Rule sử dụng AI-FIRST-ARCHITECTURE.md cho AI Agent

- **AI Agent phải:**
  - Đọc file này trước khi:
    - Sinh code cho service mới,
    - Thêm module mới vào service. [file:5]
  - Tuân thủ cấu trúc thư mục, dependency rule, pattern event/DB/API/observability. [file:5]

- **AI Agent được:**
  - Tự sinh code boilerplate theo pattern (handler, DTO, repo, adapter, test).
  - Đề xuất refactor để align với pattern này.

- **AI Agent không được:**
  - Thay đổi pattern kiến trúc (layout, dependency rule) nếu không có yêu cầu explicit từ con người.
  - Tự ý bỏ qua logging/tracing/metrics cho feature mới mà không lý do.