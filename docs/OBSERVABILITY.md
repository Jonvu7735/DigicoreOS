# OBSERVABILITY.md

## 1. Mục tiêu & Phạm vi

### Mục tiêu

Tài liệu này định nghĩa **chiến lược Observability** (Khả năng quan sát hệ thống) cho nền tảng Rust SaaS AI-First, bao gồm: [file:4]

- Logs (nhật ký hệ thống),
- Metrics (chỉ số đo lường),
- Traces (vết request). [file:4]

Mục tiêu:

- Giúp Dev, SRE, AI Agent nhìn thấy trạng thái hệ thống theo thời gian thực. [file:4]
- Chuẩn hoá cách:
  - ghi log,
  - đo metrics,
  - trace request giữa các service. [file:4]
- Là nền tảng cho:
  - Alerting,
  - Incident analysis,
  - Hiệu chỉnh hiệu năng. [file:4]

### Phạm vi

- File này là chuẩn cho:
  - Format log JSON,
  - Metrics Prometheus,
  - OpenTelemetry tracing,
  - Dashboard & alert. [file:4]
- Không mô tả:
  - Chiến lược dữ liệu tổng thể → xem `DATA-STRATEGY.md`. [file:7]
  - Flow business → xem `ARCHITECTURE.md`. [file:6]

---

## 2. Kiến trúc Observability tổng quan

### 2.1. Thành phần chính

- Logs:
  - Thu gom tập trung (Loki, ELK, ClickHouse, Cloud Logging). [file:4]
- Metrics:
  - Prometheus (scrape /metrics) + Grafana dashboard. [file:4]
- Traces:
  - OpenTelemetry (OTel) + backend Tempo/Jaeger/Cloud Trace. [file:4]

### 2.2. Nguyên tắc chung

- Mỗi service Rust phải: [file:4][file:5]
  - Dùng structured logging (log JSON key-value),
  - Expose metrics để Prometheus scrape,
  - Propagate trace context (W3C `traceparent`) qua HTTP & event. [file:4]
- Mọi environment (dev/staging/prod) dùng **cùng chuẩn**, khác nhau chủ yếu ở backend (local vs cloud). [file:4]

---

## 3. Logging (ghi log)

### 3.1. Mục tiêu logging

- Cho phép:
  - Debug lỗi trong từng service,
  - Trace 1 request end-to-end kết hợp với traces,
  - Phân tích hành vi user/hệ thống kết hợp metrics. [file:4]

### 3.2. Công cụ & Format

- Sử dụng: [file:4][file:5]
  - `tracing` + `tracing-subscriber` (Rust),
  - `tracing-opentelemetry` để bridge sang OTel (nếu cần). [file:5]
- Format log:
  - JSON, dễ parse, dễ index. [file:4][file:5]

**Fields tối thiểu** cho mỗi log entry: [file:4]

- `timestamp` (ISO8601),
- `level` (`TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`),
- `service` (`auth-svc`, `core-erp-svc`, `crm-svc`, `hrm-svc`, `reporting-svc`, `ai-svc`),
- `env` (dev/staging/prod),
- `tenant_id` (nếu có),
- `user_id` (nếu có),
- `trace_id`, `span_id` (để liên kết với traces),
- `message`,
- `fields` (context bổ sung). [file:4][file:5]

### 3.3. Phân tầng log level

- `TRACE`:
  - Rất chi tiết, dùng khi debug sâu. [file:4]
- `DEBUG`:
  - Chi tiết cho dev, thường tắt ở prod. [file:4]
- `INFO`:
  - Luồng chính, sự kiện quan trọng (order created, payment success, user login). [file:4]
- `WARN`:
  - Bất thường nhưng chưa lỗi nặng (timeout, retry, data lạ). [file:4]
- `ERROR`:
  - Lỗi nghiệp vụ/kỹ thuật cần điều tra. [file:4]

### 3.4. Nguyên tắc log an toàn

- Không log:
  - Password,
  - Token (JWT, refresh, API key),
  - Dữ liệu PII nhạy cảm chưa mask. [file:1][file:4]
- Có thể log:
  - ID, hash, metadata không nhạy cảm. [file:4]

---

## 4. Metrics (chỉ số đo lường)

### 4.1. Mục tiêu metrics

- Theo dõi:
  - Sức khoẻ hệ thống (latency, error rate, throughput),
  - KPI nghiệp vụ (order, revenue, failed payments, funnel CRM, attendance). [file:4][file:7][file:6]
- Cung cấp dữ liệu cho:
  - Alerting,
  - Dashboard Grafana. [file:4]

### 4.2. Công cụ & Chuẩn

- Prometheus format: [file:4]
  - Sử dụng crate Prometheus/metrics exporter cho Rust. [file:5]
- Mỗi service expose endpoint:
  - `GET /metrics` cho Prometheus scrape (chỉ internal). [file:4]

### 4.3. Metrics chuẩn cho mọi service

- HTTP metrics: [file:4][file:5]
  - `http_requests_total{service, method, path, status}`
  - `http_request_duration_seconds_bucket{service, method, path}`
- Error metrics:
  - `errors_total{service, type}` (vd: `db_error`, `external_api_error`). [file:4]
- Resource metrics (nếu không dùng node/container exporter):
  - `service_memory_usage_bytes{service}`
  - `service_cpu_usage_seconds_total{service}` [file:4]

### 4.4. Metrics business gợi ý

- ERP:
  - `orders_created_total{tenant_id}`
  - `payments_failed_total{tenant_id}` [file:4][file:7][file:6]
- CRM:
  - `crm_deals_won_total{tenant_id}` [file:4][file:6]
- HRM:
  - `hrm_attendance_missing_total{tenant_id}` [file:4][file:6]
- Dùng cho:
  - Dashboard business,
  - Alert khi có spike lỗi/giảm doanh thu bất thường. [file:4]

---

## 5. Tracing (vết request)

### 5.1. Mục tiêu

- Nhìn thấy đường đi của 1 request qua nhiều service (`frontend → gateway → auth → erp → reporting → ai`). [file:4][file:6]
- Phát hiện bottleneck (điểm nghẽn), timeout, retry. [file:4]
- Hỗ trợ điều tra sự cố (incident analysis). [file:4]

### 5.2. Công cụ

- OpenTelemetry (OTel):
  - `opentelemetry` + `tracing-opentelemetry` cho Rust. [file:4][file:5]
- Backend:
  - Jaeger/Tempo (local),
  - Cloud Trace (cloud). [file:4]

### 5.3. Truyền trace context

- HTTP: [file:4]
  - Bắt & truyền header `traceparent`, `tracestate` giữa các service.
- Event (NATS/Kafka):
  - Embed `trace_id` (và `span_id` nếu cần) trong metadata. [file:4][file:5]

### 5.4. Span chuẩn

- Mỗi request HTTP:
  - Root span `http.server` với attributes:
    - `http.method`, `http.route`, `http.status_code`, `tenant_id`, `user_id`. [file:4]
- Mỗi call DB:
  - Child span `db.query` với:
    - `db.system`, `db.statement` (có thể redact), duration. [file:4]
- Mỗi call external:
  - Child span `http.client` với:
    - `peer.service`, `http.url` (mask nếu cần), `status_code`. [file:4]

---

## 6. Quan hệ giữa Observability, EVENTS & DATA-STRATEGY

### 6.1. Với EVENTS.md

- Event business (UserRegistered, OrderCreated, …) là dữ liệu nghiệp vụ. [file:4][file:7][file:6]
- Observability (log/metrics/traces) là dữ liệu hệ thống. [file:4]
- Cả hai kết hợp:
  - Cho phép AI Agent hiểu:
    - Event gì xảy ra,
    - Hệ thống phản ứng thế nào (log/trace/metrics). [file:4][file:7]

### 6.2. Với DATA-STRATEGY.md

- Log/metrics/traces thuộc **nhóm Event Time-series data**. [file:7][file:4]
- Có thể đưa vào:
  - TSDB (Prometheus, Loki, ClickHouse),
  - Data lake/warehouse để phân tích dài hạn. [file:7][file:4]

---

## 7. Dashboard & Alerting

> Triển khai cụ thể nằm ở `deploy/observability/`: Prometheus scrape config,
> Grafana dashboard `digicore-overview`, và `PrometheusRule` alerts — bám theo
> đúng các metric service đang phát ra.

### 7.1. Dashboard gợi ý

- System health dashboard:
  - Latency, error rate, throughput từng service. [file:4]
- Business KPI dashboard:
  - Orders created, revenue, failed payments, CRM funnel, attendance. [file:4][file:6][file:7]
- AI performance dashboard (`ai-svc`):
  - Số request AI,
  - Latency & error rate,
  - Phân bổ model,
  - Tỷ lệ insight được user tương tác. [file:4][file:6][file:7]

### 7.2. Alerting

- Ví dụ alert: [file:4]
  - Error rate X% trong Y phút cho một service,
  - P95 latency vượt ngưỡng cho endpoint quan trọng (login, tạo order),
  - Không có order mới trong Z giờ (có thể là sự cố hệ thống hoặc bất thường business),
  - AI error tăng đột biến (timeout, quota, model fail). [file:4]

- Kênh gửi alert:
  - Email, Slack, PagerDuty, v.v. [file:4]

---

## 8. Hướng dẫn cho Dev & AI Agent

### 8.1. Khi tạo service Rust mới

- Bắt buộc: [file:4][file:5]
  - Thiết lập logging JSON + tracing,
  - Expose `/metrics` cho Prometheus,
  - Tích hợp OpenTelemetry để tạo span. [file:5]

### 8.2. Khi thêm feature mới

- Thêm:
  - Log `INFO` cho sự kiện nghiệp vụ quan trọng,
  - Metrics nếu liên quan KPI business,
  - Span cho đoạn code có nguy cơ chậm/quan trọng. [file:4][file:5]

### 8.3. Khi debug sự cố

- Quy trình:
  1. Bắt đầu từ dashboard metrics (error/latency). [file:4]
  2. Drill-down vào traces chi tiết. [file:4]
  3. Dùng log structured để tìm root cause. [file:4]

### 8.4. AI Agent

- **AI Agent phải:**
  - Dùng `OBSERVABILITY.md` làm chuẩn:
    - Format log,
    - Metrics,
    - Tracing. [file:4][file:5]
- **AI Agent được:**
  - Tự thêm log/metrics/traces theo pattern chuẩn,
  - Sinh code exporter metrics, setup logging, OTel. [file:5]
- **AI Agent không được:**
  - Sửa format log/metrics/traces chuẩn mà không có chỉ thị rõ ràng và cập nhật doc.
  - Log dữ liệu nhạy cảm (password/token/PII chưa mask). [file:1][file:4]