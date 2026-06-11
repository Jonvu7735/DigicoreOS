# SERVICE-ai-svc.md

## 1. Mục tiêu & Domain

**Service**: `ai-svc` [file:6][file:5]

**Domain chính**:

- AI Kernel cho nền tảng:
  - LLM integration,
  - Vector search / RAG,
  - Insight, dự báo, scoring,
  - Anomaly detection. [file:6][file:7]

**Trách nhiệm**:

- Cung cấp API AI dùng data ERP/CRM/HRM/Reporting. [file:6][file:7]
- Phát `AiInsightGenerated` để surface insight lên reporting/notification. [file:6]

---

## 2. API chính (qua API Gateway)

Base path: `/api/v1/ai`. [file:2]

- Query & Assistant:
  - `POST /api/v1/ai/query` – Q&A tổng quát. [file:2][file:6]
  - `POST /api/v1/ai/insight` – sinh insight. [file:2][file:6]
  - `POST /api/v1/ai/assist` – trợ lý theo ngữ cảnh màn hình. [file:2][file:6]

- Management:
  - `GET /api/v1/ai/models`
  - `POST /api/v1/ai/models/reload` [file:2][file:6]

---

## 3. Event publish/subscribe

Theo `ARCHITECTURE.md` & `EVENTS.md`: [file:6]

**Publish**

- `AiInsightGenerated` [file:6]

**Subscribe**

- Event từ:
  - `core-erp-svc` (OrderCreated, OrderPaid, StockAdjusted),
  - `crm-svc` (CustomerCreated, DealCreated, DealStageChanged),
  - `hrm-svc` (EmployeeHired, AttendanceRecorded),
  - `reporting-svc` (ReportSnapshotCreated). [file:6][file:7]

---

## 4. Data & Storage

- Vector store:
  - Giai đoạn đầu: Postgres + pgvector,
  - Sau: Qdrant/Milvus/Pinecone. [file:7]
- Metadata:
  - Bảng riêng (schema AI) cho:
    - cấu hình model,
    - prompt,
    - mapping source ↔ embedding. [file:7]
- File lớn:
  - Lưu ở object storage (S3/GCS/MinIO). [file:7]

---

## 5. Liên kết tài liệu

- `ARCHITECTURE.md` → mục 3.6 ai-svc. [file:6]
- `AI-FIRST-ARCHITECTURE.md`. [file:5]
- `API-GATEWAY.md` → `/api/v1/ai`. [file:2]
- `DATA-STRATEGY.md` → lớp AI Semantic & File. [file:7]
- `EVENTS.md` → AiInsightGenerated & các event AI liên quan.

---

## 6. Rule cho Dev & AI Agent

- Không giấu logic nghiệp vụ domain trong `ai-svc`:
  - AI chỉ nên “dùng” dữ liệu và contract từ service khác. [file:5][file:7]
- Mọi feature AI mới:
  - Phải định nghĩa rõ:
    - Data source (state/event/reporting),
    - Cách log/đánh giá kết quả (observability). [file:4][file:7]