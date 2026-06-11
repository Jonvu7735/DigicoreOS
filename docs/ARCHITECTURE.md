# ARCHITECTURE.md

## 1. Mục tiêu & Phạm vi

### Mục tiêu

Tài liệu này mô tả **kiến trúc tổng thể** của nền tảng Rust SaaS AI-First, bao gồm:

- Danh sách các service chính và **trách nhiệm** của từng service. [file:6]
- Cách các service giao tiếp với nhau qua **HTTP** (đồng bộ) và **event bus** (bất đồng bộ). [file:6]
- Cách dữ liệu chảy qua hệ thống ở mức **system view** (không đi vào chi tiết schema). [file:6][file:7]

### Phạm vi

Tài liệu này **không** mô tả:

- Layout code, module Rust, crate, migration, pattern implementation chi tiết  
  → xem `AI-FIRST-ARCHITECTURE.md`. [file:5]
- Chiến lược dữ liệu 4 lớp, 6 nhóm, mapping “dữ liệu nào lưu ở đâu”  
  → xem `DATA-STRATEGY.md`. [file:7]
- Danh sách endpoint HTTP chi tiết  
  → xem `API-GATEWAY.md`. [file:2]
- Flow auth login/refresh/logout, JWT, RBAC runtime  
  → xem `AUTH-FLOW.md`. [file:3]
- Ma trận RBAC & policy bảo mật  
  → xem `SECURITY.md`. [file:1]
- Chiến lược log/metrics/traces, dashboard, alerting  
  → xem `OBSERVABILITY.md`. [file:4]

---

## 2. Tổng quan kiến trúc nền tảng

Nền tảng được xây dựng theo triết lý **AI-First + Cloud-Native**: [file:6][file:5]

- Backend:
  - Rust + Axum + Tokio.
- Frontend:
  - Next.js.
- Hạ tầng:
  - Docker + Kubernetes (kind dùng local, GKE cho production).
- Messaging:
  - NATS giai đoạn đầu, về sau có thể mở rộng Kafka hoặc Cloud Pub/Sub. [file:6][file:5]
- Database & Storage:
  - PostgreSQL (database-per-service trên shared instance). [file:5][file:7]
  - Event store (Postgres/NATS JetStream/Kafka) cho business event & audit. [file:6][file:7]
  - TSDB/log stack (Prometheus, Loki/ELK/ClickHouse) cho log/metrics/traces. [file:7][file:4]
  - Vector store (pgvector, về sau Qdrant/Milvus/Pinecone/Weaviate) cho embedding. [file:7]
  - Object storage (S3/GCS/MinIO) cho file tài liệu. [file:7]

Ở mức logic, nền tảng gồm nhiều microservice, mỗi service chịu trách nhiệm một **bounded context** nghiệp vụ rõ ràng: [file:6][file:5]

- auth-svc
- core-erp-svc
- crm-svc
- hrm-svc
- reporting-svc
- ai-svc
- frontend-app (Next.js)

Các service giao tiếp qua:

- **HTTP API**: dùng cho command/query cần phản hồi ngay. [file:6][file:2]
- **Event bus (event-driven)**: dùng cho thông báo thay đổi trạng thái nghiệp vụ, audit, analytics, AI. [file:6][file:5]

Dữ liệu trong toàn hệ thống được tổ chức theo **4 lớp kiến trúc dữ liệu** (Transactional, Event & Time-series, Analytics, AI/Semantic/File) như mô tả chi tiết trong `DATA-STRATEGY.md`. [file:7]

---

## 3. Danh sách service & trách nhiệm

### 3.1. auth-svc

**Chức năng**

- Quản lý:
  - User (người dùng),
  - Tenant (doanh nghiệp/khách hàng SaaS),
  - Role (vai trò), Permission (quyền) theo mô hình RBAC (Role-Based Access Control – phân quyền theo vai trò). [file:1][file:6]
- Cấp và xác thực JWT (JSON Web Token – token truy cập chứa claim user/tenant/role). [file:3][file:1]
- Là **identity provider chung** cho toàn nền tảng (các service khác không tự quản user). [file:6]

**Giao tiếp**

- HTTP: [file:6][file:2]
  - Nhận request từ frontend:
    - `/api/v1/auth/login`, `/register`, `/me`, …
  - Cung cấp API cho service khác:
    - Kiểm tra token,
    - Lấy thông tin user/role nếu cần.
- Event: [file:6]
  - Publish:
    - `UserRegistered`, `UserUpdated`, `TenantCreated` (chi tiết schema xem `EVENTS.md`).
  - Subscribe:
    - `EmployeeHired` từ `hrm-svc` (tự tạo user mới nếu chính sách cho phép).

**Dữ liệu**

- Postgres schema: `auth_svc`. [file:5][file:7]

---

### 3.2. core-erp-svc

**Chức năng**

- ERP lõi (Core ERP):
  - Đơn hàng (Order),
  - Tồn kho (Inventory),
  - Mua hàng (Purchasing),
  - Hoá đơn (Billing/Invoice),
  - Tài chính cơ bản (Finance lite). [file:6]

**Giao tiếp**

- HTTP: [file:6][file:2]
  - CRUD order, sản phẩm, tồn kho, hoá đơn, thanh toán.
- Event: [file:6][file:5]
  - Publish:
    - `OrderCreated`, `OrderStatusChanged`, `OrderPaid`, `StockAdjusted`, `InvoiceIssued`, …
  - Subscribe:
    - `CustomerCreated`, `CustomerUpdated` từ `crm-svc` nếu cần sync một phần dữ liệu.

**Dữ liệu**

- Postgres schema: `erp_core_svc`. [file:5][file:7]

---

### 3.3. crm-svc

**Chức năng**

- CRM (Customer Relationship Management – quản lý quan hệ khách hàng): [file:6]
  - Khách hàng (Customer),
  - Liên hệ (Contact),
  - Cơ hội bán hàng (Deal),
  - Pipeline bán hàng,
  - Hoạt động (Activity: call/email/meeting/task).

**Giao tiếp**

- HTTP: [file:6][file:2]
  - CRUD khách hàng, deal, activity, timeline khách hàng.
- Event: [file:6][file:5]
  - Publish:
    - `CustomerCreated`, `CustomerUpdated`, `DealCreated`, `DealStageChanged`.
  - Subscribe:
    - `UserRegistered` (auth-svc) để gắn owner,
    - `OrderCreated`, `OrderPaid` (core-erp-svc) để cập nhật timeline khách hàng.

**Dữ liệu**

- Postgres schema: `crm_svc`. [file:5][file:7]

---

### 3.4. hrm-svc

**Chức năng**

- HRM (Human Resource Management – quản lý nhân sự): [file:6]
  - Hồ sơ nhân sự (Employee),
  - Chấm công (Attendance),
  - Nghỉ phép (Leave),
  - Tính lương (Payroll – giai đoạn sau).

**Giao tiếp**

- HTTP: [file:6][file:2]
  - CRUD employee, attendance, leave.
- Event: [file:6]
  - Publish:
    - `EmployeeHired`, `EmployeeTerminated`, `AttendanceRecorded`.
  - Subscribe:
    - `UserRegistered`, `TenantCreated` nếu cần sync thông tin employee ↔ user.

**Dữ liệu**

- Postgres schema: `hrm_svc`. [file:5][file:7]

---

### 3.5. reporting-svc

**Chức năng**

- Nhận event từ các service nghiệp vụ, xây dựng: [file:6][file:7]
  - Bảng fact/dimension/snapshot (mô hình data warehouse logic),
  - Dashboard tổng quan (overview, sales, inventory, CRM funnel, HRM summary),
  - Báo cáo chi tiết + export (CSV/Excel/PDF).
- Cung cấp API cho frontend và `ai-svc` lấy dữ liệu báo cáo. [file:6]

**Giao tiếp**

- HTTP: [file:6][file:2]
  - `/api/v1/reporting/overview`, `/sales-summary`, `/inventory-summary`, `/crm-funnel`, `/hrm-summary`, `/orders`, `/customers`, `/employees`, `/export`, …
- Event: [file:6][file:7]
  - Publish:
    - `ReportSnapshotCreated`.
  - Subscribe:
    - Event quan trọng từ `auth-svc`, `core-erp-svc`, `crm-svc`, `hrm-svc`, `ai-svc`.

**Dữ liệu**

- Postgres schema: `reporting_svc` (fact/dimension/snapshot). [file:6][file:7]
- Về sau có thể sync sang warehouse/lake (BigQuery, Snowflake, ClickHouse…). [file:7]

---

### 3.6. ai-svc

**Chức năng**

- Đóng vai trò “AI kernel” cho toàn nền tảng: [file:6][file:5]
  - Tích hợp LLM,
  - Tạo embedding,
  - RAG (retrieval-augmented generation – sinh câu trả lời có truy xuất dữ liệu),
  - Sinh insight (phân tích, cảnh báo, dự báo),
  - Scoring (điểm lead, rủi ro, bất thường).

**Giao tiếp**

- HTTP: [file:2][file:6]
  - `/api/v1/ai/query`: Q&A tổng quát trên nền dữ liệu.
  - `/api/v1/ai/insight`: sinh insight cho dashboard/quản trị.
  - `/api/v1/ai/assist`: trợ lý ngữ cảnh cho từng màn hình nghiệp vụ.
  - `/api/v1/ai/models`, `/api/v1/ai/models/reload`: quản lý model/prompt.
- Event: [file:6][file:5]
  - Publish:
    - `AiInsightGenerated` (insight mới cho reporting/frontend).
  - Subscribe:
    - Event từ ERP/CRM/HRM/Reporting để học, scoring, anomaly detection.

**Dữ liệu**

- Vector store (pgvector, Qdrant/Milvus/…), metadata AI. [file:7]
- Postgres schema riêng cho config/prompt/logic nếu cần. [file:5][file:7]

---

### 3.7. frontend-app (Next.js)

**Chức năng**

- Ứng dụng web Next.js (có thể là 1 app chính hoặc nhiều app con) cho người dùng doanh nghiệp. [file:6]

**Giao tiếp**

- HTTP:
  - Gọi API Gateway (Ingress) → route tới các service backend. [file:6][file:2]

**Dữ liệu**

- Không lưu dữ liệu dài hạn, chủ yếu:
  - state trên client,
  - cache ngắn hạn.

---

## 4. Giao tiếp giữa các service

### 4.1. HTTP synchronous (đồng bộ)

**Mục tiêu**

- Xử lý các lệnh (command) và truy vấn (query) cần phản hồi ngay. [file:6][file:5]

**Ví dụ chính**

- `frontend-app` → `auth-svc`:
  - login, refresh token, lấy thông tin user. [file:6][file:3]
- `frontend-app` → `core-erp-svc`:
  - tạo order, xem chi tiết order, tồn kho. [file:6][file:2]
- `frontend-app` → `crm-svc`:
  - quản lý khách hàng, deal, activity. [file:6][file:2]
- `frontend-app` → `hrm-svc`:
  - quản lý nhân sự, chấm công, nghỉ phép. [file:6][file:2]
- `frontend-app` → `reporting-svc`:
  - xem dashboard, báo cáo chi tiết, export. [file:6][file:2]
- `frontend-app` → `ai-svc`:
  - query/insight/assist. [file:6][file:2]

**Service-to-service HTTP (hạn chế)**

- `core-erp-svc` → `auth-svc`: xác thực/quyền user trong trường hợp đặc biệt. [file:6]
- `crm-svc` → `core-erp-svc`: lấy danh sách đơn hàng của khách nếu không dùng denormalized view. [file:6]

**Contract**

- Chi tiết endpoint, method, path → `API-GATEWAY.md`. [file:2]
- DTO & error format → `AI-FIRST-ARCHITECTURE.md`. [file:5]

---

### 4.2. Event asynchronous (event-driven)

**Stack**

- NATS (giai đoạn đầu) làm event bus. [file:6][file:5]
- Sau này có thể nâng cấp:
  - Kafka,
  - Pub/Sub,
  - EventStoreDB.

**Nguyên tắc**

- Mỗi service:
  - Publish event sau khi commit transaction thành công (outbox pattern). [file:6][file:7]
  - Subscribe event cần thiết cho domain của mình.
- Schema event:
  - Được định nghĩa tập trung trong `EVENTS.md` và crate `event-models`. [file:5][file:6]

**Lợi ích**

- Giảm coupling:
  - Service phát `OrderCreated` không cần biết ai sẽ nghe. [file:6]
- Dễ mở rộng:
  - Thêm service mới chỉ cần subscribe event.
- Là nguồn dữ liệu chuẩn cho:
  - Audit & trace nghiệp vụ,
  - Reporting & BI,
  - AI training & inference. [file:7]

---

## 5. Dòng chảy dữ liệu tiêu biểu

### 5.1. Luồng tạo đơn hàng

1. User thao tác trên frontend → gửi `POST /api/v1/erp/orders` đến API Gateway. [file:6][file:2]
2. Gateway route vào `core-erp-svc`. [file:2][file:6]
3. `core-erp-svc`:
   - Validate input, áp dụng rule nghiệp vụ. [file:6]
   - Ghi state order vào Postgres (`erp_core_svc`). [file:7]
   - Ghi event `OrderCreated` vào outbox/event store. [file:7]
4. Worker/event publisher:
   - Publish `OrderCreated` lên NATS với payload đúng schema `EVENTS.md`. [file:6][file:7]
5. Các service khác:
   - `reporting-svc`: cập nhật `fact_orders`/snapshot. [file:6][file:7]
   - `crm-svc`: cập nhật timeline khách hàng. [file:6]
   - `ai-svc`: update feature, có thể trigger insight (ví dụ: “khách hàng này mua nhiều hơn bình thường”). [file:6][file:5]

---

### 5.2. Luồng cập nhật tồn kho

1. Một action làm thay đổi tồn kho (nhập, xuất, điều chỉnh) được gọi vào `core-erp-svc`. [file:6]
2. `core-erp-svc`:
   - Cập nhật state tồn kho trong Postgres. [file:7]
   - Publish `StockAdjusted`. [file:6][file:7]
3. `reporting-svc`:
   - Nhận event → cập nhật snapshot tồn kho. [file:6][file:7]
4. `ai-svc`:
   - Sử dụng chuỗi `StockAdjusted` theo thời gian để phát hiện bất thường (anomaly detection). [file:6][file:5][file:7]

---

### 5.3. Luồng AI Insight

1. `ai-svc` thực hiện một nhiệm vụ phân tích, sử dụng dữ liệu từ: [file:6][file:7]
   - Lớp Transactional (Postgres state),
   - Lớp Event (event store),
   - Lớp Time-series (log/metrics/traces),
   - Lớp AI Semantic (vector, file).
2. Khi tạo được insight quan trọng:
   - Publish `AiInsightGenerated`. [file:6][file:5]
3. `reporting-svc`:
   - Subscribe insight, ghi lại để hiển thị trong dashboard/báo cáo. [file:6]
4. Frontend:
   - Gọi API reporting/ai để lấy insight, hiển thị cho người dùng. [file:2][file:6]

---

## 6. Liên kết với các tài liệu kiến trúc khác

- `AI-FIRST-ARCHITECTURE.md`  
  → Layout code, pattern domain/infra/api/bootstrap, event-models, contract-first, quy trình làm việc với AI Agent. [file:5]

- `API-GATEWAY.md`  
  → Danh sách route HTTP, base path `/api/v1/<domain>/...`, versioning, multi-tenant header. [file:2]

- `DATA-STRATEGY.md`  
  → 4 lớp dữ liệu (Transactional, Event & Time-series, Analytics, AI/Semantic/File), 6 nhóm dữ liệu, mapping nơi lưu. [file:7]

- `EVENTS.md` (nếu có)  
  → Schema chuẩn cho event, subject NATS/Kafka, producer/consumer. [file:5][file:6]

- `SECURITY.md`  
  → RBAC, TLS, rate limit, input validation, data at rest/in transit, secret management. [file:1]

- `AUTH-FLOW.md`  
  → Flow login/refresh/logout, cấu trúc JWT, pipeline kiểm tra RBAC tại gateway/service. [file:3]

- `OBSERVABILITY.md`  
  → Chuẩn log JSON, metrics Prometheus, OpenTelemetry traces, dashboard, alerting. [file:4]

---

## 7. Nguyên tắc mở rộng kiến trúc

1. **Thêm service mới**

   - Ví dụ: `helpdesk-svc`, `workflow-svc`, `notification-svc`.
   - Yêu cầu:
     - Xác định rõ bounded context & domain nghiệp vụ. [file:6][file:5]
     - Tuân thủ template service Rust trong `AI-FIRST-ARCHITECTURE.md`. [file:5]
     - Có schema riêng trong Postgres (nếu cần state). [file:7]
     - Định nghĩa event mới (nếu publish) trong `EVENTS.md`. [file:5][file:7]
     - Cập nhật `API-GATEWAY.md` nếu expose API. [file:2]

2. **Tránh coupling mạnh giữa service**

   - Ưu tiên giao tiếp qua event (asynchronous). [file:6][file:5]
   - HTTP service-to-service chỉ dùng khi thực sự cần response đồng bộ.

3. **Thiết kế AI-friendly**

   - Naming & structure phải nhất quán giữa:
     - API, event, schema DB, log/metrics/traces. [file:5][file:4][file:7]
   - Mỗi khi thêm/đổi schema/event/API/log format:
     - Phải cập nhật file doc tương ứng (ARCHITECTURE, AI-FIRST, DATA-STRATEGY, API-GATEWAY, EVENTS, OBSERVABILITY). [file:6][file:5][file:7][file:2][file:4]

4. **Chuẩn bị cho scale**

   - Mỗi service:
     - Triển khai độc lập trên Kubernetes (Deployment + HPA). [file:6]
   - Data layer:
     - Có thể scale ra warehouse/lake/vector DB mà không phá vỡ contract hiện tại. [file:7]

---

## 8. Rule sử dụng ARCHITECTURE.md cho AI Agent

- **AI Agent phải làm trước khi sinh code/thiết kế:**
  - Đọc `ARCHITECTURE.md` để hiểu:
    - Service nào chịu trách nhiệm domain nào.
    - Flow chính (order, inventory, insight…). [file:6]
  - Kết hợp với:
    - `API-GATEWAY.md` để biết route. [file:2]
    - `DATA-STRATEGY.md` để biết nơi lưu dữ liệu. [file:7]
    - `EVENTS.md` để biết event nào cần dùng. [file:5][file:7]

- **AI Agent được phép:**
  - Đề xuất thêm event/flow/service **nếu** có mô tả rõ ràng trong issue/spec từ con người.
  - Sinh code cho service đã xác định trong doc này.

- **AI Agent không được:**
  - Tự ý di chuyển logic nghiệp vụ từ service này sang service khác nếu không có yêu cầu rõ ràng.
  - Tự tạo thêm service mới mà không cập nhật (hoặc yêu cầu cập nhật) `ARCHITECTURE.md`.
  - Thay đổi flow business quan trọng (order, inventory, auth, insight) mà không có phê duyệt của con người.