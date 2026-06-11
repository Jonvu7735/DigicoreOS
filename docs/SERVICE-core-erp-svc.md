# SERVICE-core-erp-svc.md

## 1. Mục tiêu & Domain

**Service**: `core-erp-svc` [file:6][file:5]

**Domain chính**:

- ERP lõi:
  - Đơn hàng (Order),
  - Tồn kho (Inventory),
  - Hoá đơn (Invoice),
  - Thanh toán (Payment),
  - Finance cơ bản. [file:6]

**Trách nhiệm**:

- Quản lý lifecycle của đơn hàng và tồn kho. [file:6]
- Phát sinh event cho reporting, CRM, AI (OrderCreated, OrderPaid, StockAdjusted, …). [file:6][file:7]

---

## 2. API chính (qua API Gateway)

Base path: `/api/v1/erp`. [file:2]

Một số endpoint tiêu biểu:

- Orders:
  - `GET /api/v1/erp/orders`
  - `POST /api/v1/erp/orders`
  - `GET /api/v1/erp/orders/{order_id}`
  - `POST /api/v1/erp/orders/{order_id}/confirm`
  - `POST /api/v1/erp/orders/{order_id}/complete`
  - `POST /api/v1/erp/orders/{order_id}/cancel` [file:2][file:6]

- Payments:
  - `POST /api/v1/erp/orders/{order_id}/payments`
  - `GET /api/v1/erp/orders/{order_id}/payments` [file:2]

- Inventory:
  - `GET /api/v1/erp/inventory`
  - `POST /api/v1/erp/inventory/adjustments` [file:2]

- Products:
  - `GET/POST/PATCH/DELETE /api/v1/erp/products/...` [file:2]

- Invoices:
  - `GET/POST /api/v1/erp/invoices`
  - `GET /api/v1/erp/invoices/{invoice_id}`
  - `POST /api/v1/erp/invoices/{invoice_id}/cancel` [file:2]

---

## 3. Event publish/subscribe

Theo `ARCHITECTURE.md` & `EVENTS.md`: [file:6]

**Publish**

- `OrderCreated`
- `OrderStatusChanged`
- `OrderPaid`
- `StockAdjusted`
- `InvoiceIssued` [file:6]

**Subscribe**

- `CustomerCreated`, `CustomerUpdated` (từ `crm-svc`) – nếu cần sync một phần thông tin khách. [file:6]

---

## 4. Data & Storage

- DB: PostgreSQL, schema: `erp_core_svc` (hoặc `erp_core`). [file:5][file:7]
- Bảng chính (gợi ý):
  - `orders`, `order_items`,
  - `products`,
  - `inventory_balances`, `inventory_adjustments`,
  - `invoices`, `payments`. [file:7]

Chiến lược “state vs event vs reporting” → `DATA-STRATEGY.md`. [file:7]

---

## 5. Liên kết tài liệu

- Kiến trúc tổng:
  - `ARCHITECTURE.md` → mục 3.2 core-erp-svc. [file:6]
- Code layout:
  - `AI-FIRST-ARCHITECTURE.md`. [file:5]
- API:
  - `API-GATEWAY.md` → nhóm `/api/v1/erp`. [file:2]
- Data:
  - `DATA-STRATEGY.md`. [file:7]
- Events:
  - `EVENTS.md` → nhóm ERP events.

---

## 6. Rule cho Dev & AI Agent

- Chỉ implement nghiệp vụ ERP lõi tại đây.
- Mọi reporting phức tạp nên đẩy sang `reporting-svc` qua event. [file:6]
- Khi thêm tính năng mới (VD: new order status):
  - Cập nhật:
    - schema DB (`DATA-STRATEGY.md`),
    - event tương ứng (`EVENTS.md`),
    - API (`API-GATEWAY.md`),
    - tài liệu service này.