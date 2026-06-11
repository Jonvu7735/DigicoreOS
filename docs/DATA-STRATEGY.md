# DATA-STRATEGY.md

## 1. Mục tiêu & Phạm vi

### Mục tiêu

Tài liệu này mô tả **chiến lược dữ liệu** cho nền tảng Rust SaaS AI-First với 3 mục tiêu chính: [file:7]

1. Cấu trúc tổng thể:
   - Dữ liệu được phân lớp rõ ràng, dễ mở rộng khi thêm sản phẩm (ERP, CRM, HRM, BI, AI-svc, …). [file:7]
2. Đảm bảo toàn vẹn dữ liệu:
   - Từ transaction đến event, log, analytics, AI đều traceable, kiểm soát được, khôi phục được. [file:7]
3. Tối ưu cho AI:
   - Dữ liệu được tổ chức để AI Agent/LLM dễ truy cập, hiểu, học & hành động. [file:7]

### Phạm vi

- Đây là **nguồn chuẩn** cho:
  - 4 lớp kiến trúc dữ liệu,
  - 6 nhóm dữ liệu,
  - Mapping “dữ liệu nào lưu ở đâu”. [file:7]
- Không mô tả:
  - Flow business chi tiết → xem `ARCHITECTURE.md`. [file:6]
  - Layout code → xem `AI-FIRST-ARCHITECTURE.md`. [file:5]

---

## 2. Kiến trúc dữ liệu tổng thể

### 2.1. 4 lớp dữ liệu

1. **Lớp Transactional (System of Record)** [file:7]
   - Lưu trạng thái hiện tại của nghiệp vụ (state).
   - Công nghệ chính:
     - PostgreSQL, database-per-service (schema per service). [file:7][file:5]

2. **Lớp Event & Time-series / Log** [file:7]
   - Lưu mọi thay đổi nghiệp vụ quan trọng (business event) & hành vi hệ thống (log/metrics/traces).
   - Công nghệ:
     - Event store (Postgres/Kafka/NATS JetStream),
     - TSDB/log stack (Prometheus, Loki/ELK/ClickHouse). [file:7][file:4]

3. **Lớp Analytics / Lakehouse** [file:7]
   - Lưu dữ liệu tổng hợp phục vụ báo cáo, dashboard, phân tích, training.
   - Công nghệ:
     - Postgres schema `reporting_svc` giai đoạn đầu,
     - Về sau: BigQuery, Snowflake, ClickHouse, DuckDB/MotherDuck, lakehouse trên object storage. [file:7]

4. **Lớp AI Semantic & File** [file:7]
   - Lưu embedding, metadata, và file/tài liệu gốc phục vụ RAG, semantic search, recommendation.
   - Công nghệ:
     - Vector store: pgvector ban đầu, Qdrant/Milvus/Pinecone về sau.
     - Object storage: S3/GCS/MinIO. [file:7]

### 2.2. 6 nhóm dữ liệu chính

1. State nghiệp vụ (Operational State). [file:7]
2. Business Events & Audit Log (sự kiện nghiệp vụ). [file:7]
3. Log kỹ thuật & Telemetry (logs, metrics, traces). [file:7][file:4]
4. Dữ liệu phân tích (BI, Analytics, Reporting). [file:7]
5. Vector AI Data (embedding). [file:7]
6. File/Tài liệu (documents, blobs). [file:7]

---

## 3. Mapping: Dữ liệu nào lưu ở đâu?

### 3.1. State nghiệp vụ (Operational State)

**Nơi lưu**

- PostgreSQL Cloud SQL với chiến lược database-per-service trên shared instance. [file:7][file:5]

**Service → schema**

| Service         | Schema gợi ý    |
|-----------------|-----------------|
| `auth-svc`      | `auth_svc`      |
| `core-erp-svc`  | `erp_core_svc`  |
| `crm-svc`       | `crm_svc`       |
| `hrm-svc`       | `hrm_svc`       |
| `reporting-svc` | `reporting_svc` |

**Nguyên tắc** [file:7][file:5]

- Chỉ lưu:
  - Trạng thái hiện tại & lịch sử cần thiết cho nghiệp vụ.
- Không lưu:
  - Log kỹ thuật,
  - Event raw,
  - Embedding,
  - File lớn,
  - Dữ liệu tạm/bộ nhớ làm việc. [file:7]
- Mọi truy cập DB phải đi qua layer `infra/db` (Repo implement trait domain). [file:5][file:7]
- Đảm bảo ACID, constraint, foreign key để giữ toàn vẹn transaction. [file:7]

### 3.2. Business Event & Audit Log

**Nơi lưu** [file:7]

- Giai đoạn đầu:
  - Postgres (bảng events append-only) hoặc NATS JetStream.
- Giai đoạn sau:
  - Kafka,
  - EventStoreDB,
  - Hoặc giải pháp chuyên cho event sourcing. [file:7]

**Cấu trúc event cơ bản (Postgres)** [file:7]

```text
events (
  id            UUID,          -- event_id
  occurred_at   TIMESTAMPTZ,
  tenant_id     TEXT,
  aggregate_type TEXT,         -- order, inventory, user, ...
  aggregate_id  TEXT,
  event_type    TEXT,          -- OrderCreated, ...
  version       INT,
  payload       JSONB          -- body event
)
```

**Nguyên tắc** [file:7]

- Event là append-only:
  - Không sửa/xoá event. Nếu logic đổi → thêm event mới (version mới). [file:7]
- State có thể rebuild từ event + snapshot (nếu áp dụng event sourcing). [file:7]
- Dùng **outbox pattern**:
  - Trong 1 transaction DB:
    - Cập nhật state + ghi event vào bảng outbox.
  - Worker:
    - Publish event từ outbox sang broker (NATS/Kafka). [file:7][file:5]

### 3.3. Log kỹ thuật, Metrics & Traces

**Nơi lưu** [file:7][file:4]

- TSDB & log stack:
  - Prometheus (metrics),
  - Loki/ELK/ClickHouse (logs),
  - Tempo/Jaeger/Cloud Trace (traces). [file:4]

**Nguyên tắc**

- Không lưu log kỹ thuật trong Postgres state. [file:7]
- Log/metrics có **retention ngắn hơn** (30–180 ngày) so với dữ liệu nghiệp vụ. [file:7][file:4]
- Đây là nguồn chính cho:
  - Monitoring & alerting,
  - AI Ops (AI phân tích hành vi hệ thống). [file:4][file:7]

### 3.4. Dữ liệu phân tích (BI/Analytics/Reporting)

**Nơi lưu** [file:7]

- Giai đoạn đầu:
  - Postgres schema `reporting_svc`:
    - Bảng fact/dimension,
    - Aggregate,
    - Materialized view. [file:7][file:6]
- Về sau:
  - Data warehouse/lake:
    - BigQuery, Snowflake, ClickHouse, DuckDB/MotherDuck, lakehouse trên object storage. [file:7]

**Nguồn dữ liệu**

- Event store (stream/batch từ event).
- State DB (khi cần join). [file:7]

**Nguyên tắc**

- Bảng reporting được tối ưu cho đọc (read-optimized), không dùng cho transaction. [file:7]
- Có thể rebuild từ state + event (không phải system of record duy nhất). [file:7]

### 3.5. Vector AI Data (Semantic Embedding)

**Nơi lưu** [file:7]

- Giai đoạn đầu:
  - PostgreSQL + pgvector. [file:7]
- Sau này:
  - Vector DB chuyên dụng:
    - Qdrant, Milvus, Pinecone, Weaviate, v.v. [file:7]

**Dữ liệu**

- Embedding cho:
  - Tài liệu ERP (order, invoice, note),
  - Note CRM, email, lịch sử tương tác,
  - Log/hành vi,
  - Knowledge base, FAQ, help. [file:7]

- Metadata:
  - `source_type`, `source_id`, `tenant_id`, `version`, `lang`, `permissions`. [file:7]

**Nguyên tắc**

- Nội dung gốc (PDF, DOCX, text) lưu ở object storage.
- Vector chỉ lưu embedding + metadata → dễ trace về nguồn gốc. [file:7]
- Phân vùng/partition theo tenant để đảm bảo isolation & security. [file:7][file:1]

### 3.6. File & Tài liệu (Documents/Blobs)

**Nơi lưu** [file:7]

- Object storage:
  - GCS, S3, MinIO, Ceph, … [file:7]
- Trong DB:
  - Chỉ lưu metadata:
    - `file_id`, `path/key`, `file_type`, `size`, `checksum/hash`, `tenant_id`, link tới bản ghi nghiệp vụ. [file:7]

**Nguyên tắc**

- Không lưu file lớn thẳng vào Postgres (BYTEA) trừ khi rất đặc biệt. [file:7]
- Chính sách retention (thời gian lưu) có thể tuỳ loại:
  - Hoá đơn, hợp đồng, tài liệu support, v.v. [file:7]

---

## 4. Toàn vẹn dữ liệu từ Transaction đến Event, Analytics & AI

### 4.1. Toàn vẹn ở lớp Transactional (Postgres)

- Sử dụng ACID, khoá, foreign key, constraint để:
  - Tránh trạng thái mồ côi (orphan),
  - Tránh vi phạm nghiệp vụ cơ bản (số dư âm, tồn kho âm, v.v.). [file:7]
- Mỗi service chịu trách nhiệm toàn vẹn trong domain của mình. [file:7][file:5]

### 4.2. Toàn vẹn ở lớp Event

- Event là bất biến, append-only. [file:7]
- Mỗi event:
  - Gắn với `aggregate_type`, `aggregate_id`, `version`, `tenant_id`, `occurred_at`. [file:7]
  - Có `event_id` duy nhất dùng để idempotent. [file:7][file:5]
- Outbox pattern:
  - Đảm bảo:
    - State và event được ghi đồng bộ trong 1 transaction,
    - Không có case update DB nhưng quên publish event. [file:7][file:5]

### 4.3. Toàn vẹn ở lớp Analytics/BI

- Data mart/reporting:
  - Luôn rebuild được từ:
    - State trong Postgres,
    - Event store/log. [file:7]
- Mỗi job ETL/ELT:
  - Ghi lại watermark/offset:
    - Đã xử lý tới event ID/timestamp nào.
  - Log trạng thái, retry nếu lỗi. [file:7]

### 4.4. Toàn vẹn ở lớp AI Semantic & File

- Mỗi embedding:
  - Luôn có tham chiếu ngược:
    - `source_id`, `source_type`, `tenant_id`, `version`. [file:7]
- Hỗ trợ:
  - Xoá/ẩn dữ liệu 1 tenant theo yêu cầu (data erasure). [file:7][file:1]
- File:
  - Lưu hash/checksum để phát hiện corruption. [file:7]

---

## 5. Tối ưu cho AI: Data, Memory, Pipeline

### 5.1. Tổ chức dữ liệu AI-friendly

- Postgres (state):
  - Nguồn sự thật (source of truth) cho nghiệp vụ.
  - Schema rõ ràng, dễ để AI sinh SQL truy vấn. [file:7][file:5]
- Event store:
  - Timeline lịch sử, AI có thể dùng để giải thích “vì sao” trạng thái hiện tại xảy ra. [file:7]
- TSDB/log:
  - Hành vi hệ thống theo thời gian, dùng cho AI Ops (nhận diện pattern lỗi, nghẽn…). [file:7][file:4]
- Vector & File:
  - Kiến thức ngữ nghĩa (semantic knowledge) dùng cho RAG. [file:7]

### 5.2. Mô hình “memory” cho AI Agent

3 cấp:

1. **Working memory** (bộ nhớ làm việc)
   - Context tạm thời trong 1 request:
     - State hiện tại liên quan,
     - Một số event gần nhất,
     - Một vài vector gần nhất. [file:7]

2. **External memory**
   - Toàn bộ:
     - Postgres state,
     - Event store,
     - TSDB,
     - Vector store,
     - File storage,
   - Truy cập qua API/service, spec OpenAPI, schema event, catalog bảng. [file:7]

3. **Scalable/long-term memory**
   - Warehouse/lake, archive event, logs, vector/file dài hạn:
     - Dùng cho phân tích dài hạn, training/retraining. [file:7]

### 5.3. Pipeline AI-native

1. Thu thập dữ liệu:
   - Từ giao dịch state & event,
   - Log/metrics/traces. [file:7][file:4]

2. Chuẩn hoá & lưu trữ:
   - ETL/ELT từ event/state → analytics (fact/dimension),
   - Tạo embedding từ tài liệu/log/state → vector store. [file:7]

3. Inference online:
   - Trong mỗi request, AI Agent:
     - Query state Postgres,
     - Lấy vector liên quan,
     - Lấy event gần nhất nếu cần,
     - Sinh câu trả lời/hành động. [file:7][file:5]

4. Feedback loop:
   - Kết quả AI:
     - Được log lại,
     - Thành dữ liệu để đánh giá, tối ưu model, điều chỉnh prompt. [file:7][file:4]

---

## 6. Rule cho Dev & AI Agent

### 6.1. Nguyên tắc vàng

1. Không nhét mọi thứ vào một DB:
   - State → Postgres schema riêng,
   - Event → event store,
   - Log/metrics → TSDB/log stack,
   - Analytics → reporting/warehouse,
   - Vector → vector DB,
   - File → object storage. [file:7]
2. Luôn phát event cho hành động nghiệp vụ quan trọng:
   - Đảm bảo:
     - Audit,
     - Replay,
     - Analytics,
     - AI đều dựa trên event chuẩn. [file:7][file:5]

### 6.2. AI Agent

- **AI Agent phải:**
  - Dùng `DATA-STRATEGY.md` để quyết định:
    - Dữ liệu mới nên lưu ở lớp nào,
    - Sử dụng công nghệ lưu trữ gì. [file:7][file:5]
- **AI Agent được:**
  - Sinh migration, schema, query, ETL job, adapter code cho DB/event/TSDB/vector DB. [file:5][file:7]
- **AI Agent không được:**
  - Tự thay đổi schema state/event/analytics mà không có review con người.
  - Log/bơm dữ liệu nhạy cảm vượt quá policy (retention, PII, tenant). [file:1][file:7]

---

## 7. Tóm tắt

- Nền tảng dữ liệu được tổ chức thành 4 lớp, 6 nhóm dữ liệu, đảm bảo:
  - Toàn vẹn nghiệp vụ,
  - Chuẩn bị tốt cho analytics & AI,
  - Dễ cho AI Agent truy cập & hiểu. [file:7][file:5]