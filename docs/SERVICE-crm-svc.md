# SERVICE-crm-svc.md

## 1. Mục tiêu & Domain

**Service**: `crm-svc` [file:6][file:5]

**Domain chính**:

- CRM (Customer Relationship Management):
  - Khách hàng (Customer),
  - Liên hệ (Contact),
  - Cơ hội (Deal),
  - Pipeline,
  - Activities (call/email/meeting/task). [file:6]

**Trách nhiệm**:

- Quản lý thông tin & lifecycle khách hàng, cơ hội. [file:6]
- Phát event CRM cho reporting & AI. [file:6][file:7]

---

## 2. API chính (qua API Gateway)

Base path: `/api/v1/crm`. [file:2]

- Customers:
  - `GET /api/v1/crm/customers`
  - `POST /api/v1/crm/customers`
  - `GET /api/v1/crm/customers/{customer_id}`
  - `PATCH /api/v1/crm/customers/{customer_id}`
  - `GET /api/v1/crm/customers/{customer_id}/timeline` [file:2][file:6]

- Deals:
  - `GET /api/v1/crm/deals`
  - `POST /api/v1/crm/deals`
  - `GET /api/v1/crm/deals/{deal_id}`
  - `PATCH /api/v1/crm/deals/{deal_id}`
  - `POST /api/v1/crm/deals/{deal_id}/move-stage` [file:2]

- Activities:
  - `GET /api/v1/crm/activities`
  - `POST /api/v1/crm/activities`
  - `GET /api/v1/crm/activities/{activity_id}` [file:2]

---

## 3. Event publish/subscribe

Theo `ARCHITECTURE.md` & `EVENTS.md`: [file:6]

**Publish**

- `CustomerCreated`, `CustomerUpdated`
- `DealCreated`, `DealStageChanged` [file:6]

**Subscribe**

- `UserRegistered` (auth-svc) – gắn owner, mapping user ↔ khách hàng nếu cần. [file:6]
- `OrderCreated`, `OrderPaid` (core-erp-svc) – update CRM timeline. [file:6]

---

## 4. Data & Storage

- DB: PostgreSQL, schema: `crm_svc` (hoặc `crm`). [file:5][file:7]
- Bảng chính (gợi ý):
  - `customers`, `contacts`,
  - `deals`, `deal_stages`,
  - `activities`, `customer_timeline`. [file:7]

---

## 5. Liên kết tài liệu

- `ARCHITECTURE.md` → mục 3.3 crm-svc. [file:6]
- `AI-FIRST-ARCHITECTURE.md`. [file:5]
- `API-GATEWAY.md` → nhóm `/api/v1/crm`. [file:2]
- `DATA-STRATEGY.md`. [file:7]
- `EVENTS.md` → CRM events.

---

## 6. Rule cho Dev & AI Agent

- Không nhét logic ERP/HRM/Reporting vào CRM.
- Mọi đồng bộ với ERP phải ưu tiên dùng event. [file:6]
- Khi thêm field/logic quan trọng:
  - Cập nhật:
    - DB schema (DATA-STRATEGY),
    - Event (EVENTS),
    - API (API-GATEWAY),
    - file này & ARCHITECTURE.