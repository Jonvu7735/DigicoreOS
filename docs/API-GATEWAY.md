# API-GATEWAY.md

## 1. Mục tiêu & Phạm vi

### Mục tiêu

Tài liệu này chuẩn hoá **bề mặt API** được expose qua API Gateway/Ingress cho nền tảng Rust SaaS AI-First, nhằm: [file:2]

- Định nghĩa các **route chính** cho từng domain:
  - Auth, ERP, CRM, HRM, Reporting, AI. [file:2]
- Làm “hợp đồng HTTP” cho:
  - Frontend (Next.js),
  - Client bên ngoài,
  - AI Agent khi gọi backend. [file:2]
- Đảm bảo:
  - naming,
  - versioning,
  - auth/multi-tenant
  nhất quán, dễ mở rộng. [file:2][file:3][file:1]

### Phạm vi

- Tài liệu này tập trung vào:
  - Cấu trúc URL,
  - HTTP method,
  - Phân nhóm theo domain. [file:2]
- Không mô tả:
  - Chi tiết schema request/response → nằm trong OpenAPI spec riêng: [`docs/openapi.yaml`](openapi.yaml).
  - Logic auth flow (login/refresh/logout) → xem `AUTH-FLOW.md`. [file:3]
  - Ma trận RBAC chi tiết → xem `SECURITY.md`. [file:1]

---

## 2. Nguyên tắc chung

### 2.1. Base URL & Versioning

- Tất cả API backend public đi qua gateway với dạng: [file:2]

```text
/api/v1/<domain>/...
```

- Ví dụ: [file:2]
  - `/api/v1/auth/login`
  - `/api/v1/erp/orders`
  - `/api/v1/crm/customers`
  - `/api/v1/hrm/employees`
  - `/api/v1/reporting/sales-summary`
  - `/api/v1/ai/query`

- Versioning: [file:2]
  - `v1` là phiên bản đầu tiên.
  - Khi có breaking change lớn → `v2`, giữ `v1` một thời gian để tương thích.

### 2.2. Auth & Multi-tenant

- Hầu hết API yêu cầu: [file:2][file:3][file:1]
  - Header `Authorization: Bearer <JWT>`.
  - Tenant:
    - Hoặc qua header `X-Tenant-Id: <tenant_id>`,
    - Hoặc embed trong JWT (tùy thiết kế cụ thể trong `AUTH-FLOW.md`). [file:3]

- Một số API public/unauthenticated:
  - `POST /api/v1/auth/login`
  - `POST /api/v1/auth/register` (nếu có)
  chỉ yêu cầu thông tin cơ bản. [file:2][file:3]

### 2.3. Quy ước REST cơ bản

- HTTP method: [file:2]
  - `GET`    → đọc dữ liệu.
  - `POST`   → tạo mới hoặc thực hiện action.
  - `PUT`    → thay thế toàn bộ resource.
  - `PATCH`  → cập nhật một phần resource.
  - `DELETE` → xoá resource.

- Query param:
  - Dùng cho filter/sort/pagination (ví dụ: `status=`, `from_date=`, `page=`, `page_size=`). [file:2]

---

## 3. Nhóm API Auth (auth-svc)

**Base path**: `/api/v1/auth` [file:2]

### 3.1. Authentication

- `POST /api/v1/auth/login` [file:2][file:3]
  - Đăng nhập, trả về JWT access/refresh.

- `POST /api/v1/auth/refresh` [file:2][file:3]
  - Refresh access token.

- `POST /api/v1/auth/logout` [file:2][file:3]
  - Đăng xuất, revoke refresh token (tuỳ implementation).

### 3.2. User & Profile

- `GET /api/v1/auth/me` [file:2]
  - Lấy thông tin user hiện tại (từ JWT).

- `GET /api/v1/auth/users`
  - Danh sách user trong tenant (có filter/pagination). [file:2]

- `POST /api/v1/auth/users`
  - Tạo user mới (dành cho admin). [file:2][file:1]

- `GET /api/v1/auth/users/{user_id}`
  - Lấy chi tiết user. [file:2]

- `PATCH /api/v1/auth/users/{user_id}`
  - Cập nhật một phần thông tin user. [file:2]

- `DELETE /api/v1/auth/users/{user_id}`
  - Vô hiệu hóa/xoá user (tuỳ chính sách). [file:2][file:1]

### 3.3. Tenant

- `GET /api/v1/auth/tenants` [file:2]
  - Danh sách tenant (chỉ cho super admin/system).

- `POST /api/v1/auth/tenants` [file:2]
  - Tạo tenant mới.

- `GET /api/v1/auth/tenants/{tenant_id}`
  - Lấy chi tiết tenant. [file:2]

- `PATCH /api/v1/auth/tenants/{tenant_id}`
  - Cập nhật thông tin tenant (plan, trạng thái, v.v.). [file:2]

---

## 4. Nhóm API ERP lõi (core-erp-svc)

**Base path**: `/api/v1/erp` [file:2][file:6]

### 4.1. Orders

- `GET /api/v1/erp/orders`
  - List orders (filter theo status, date, customer, pagination...). [file:2]

- `POST /api/v1/erp/orders`
  - Tạo order mới. [file:2]

- `GET /api/v1/erp/orders/{order_id}`
  - Lấy chi tiết order. [file:2]

- `PATCH /api/v1/erp/orders/{order_id}`
  - Cập nhật một phần order (ghi chú, thông tin bổ sung...). [file:2]

- `POST /api/v1/erp/orders/{order_id}/confirm`
  - Xác nhận order. [file:2]

- `POST /api/v1/erp/orders/{order_id}/complete`
  - Đánh dấu order hoàn tất. [file:2]

- `POST /api/v1/erp/orders/{order_id}/cancel`
  - Huỷ order. [file:2]

### 4.2. Payments

- `POST /api/v1/erp/orders/{order_id}/payments`
  - Ghi nhận thanh toán cho order. [file:2]

- `GET /api/v1/erp/orders/{order_id}/payments`
  - Danh sách các khoản thanh toán của order. [file:2]

### 4.3. Inventory

- `GET /api/v1/erp/inventory`
  - Xem tồn kho theo sản phẩm/kho. [file:2]

- `POST /api/v1/erp/inventory/adjustments`
  - Tạo phiếu điều chỉnh tồn kho. [file:2]

- `GET /api/v1/erp/inventory/adjustments`
  - List phiếu điều chỉnh. [file:2]

### 4.4. Products

- `GET /api/v1/erp/products`
  - Danh sách sản phẩm. [file:2]

- `POST /api/v1/erp/products`
  - Tạo sản phẩm mới. [file:2]

- `GET /api/v1/erp/products/{product_id}`
  - Chi tiết sản phẩm. [file:2]

- `PATCH /api/v1/erp/products/{product_id}`
  - Cập nhật sản phẩm. [file:2]

- `DELETE /api/v1/erp/products/{product_id}`
  - Ngừng kinh doanh/xoá sản phẩm (tuỳ chính sách). [file:2][file:1]

### 4.5. Invoices

- `GET /api/v1/erp/invoices`
  - Danh sách hoá đơn. [file:2]

- `POST /api/v1/erp/invoices`
  - Tạo hoặc phát hành hoá đơn (tuỳ thiết kế). [file:2]

- `GET /api/v1/erp/invoices/{invoice_id}`
  - Chi tiết hoá đơn. [file:2]

- `POST /api/v1/erp/invoices/{invoice_id}/cancel`
  - Huỷ hoá đơn. [file:2]

---

## 5. Nhóm API CRM (crm-svc)

**Base path**: `/api/v1/crm` [file:2][file:6]

### 5.1. Customers

- `GET /api/v1/crm/customers`
  - List khách hàng (filter theo segment, owner, text search...). [file:2]

- `POST /api/v1/crm/customers`
  - Tạo khách hàng mới. [file:2]

- `GET /api/v1/crm/customers/{customer_id}`
  - Chi tiết khách hàng. [file:2]

- `PATCH /api/v1/crm/customers/{customer_id}`
  - Cập nhật thông tin khách hàng. [file:2]

- `GET /api/v1/crm/customers/{customer_id}/timeline`
  - Timeline hoạt động (orders, deals, tương tác...). [file:2][file:6]

### 5.2. Deals

- `GET /api/v1/crm/deals`
  - Danh sách cơ hội bán hàng (deal) theo pipeline. [file:2]

- `POST /api/v1/crm/deals`
  - Tạo deal mới. [file:2]

- `GET /api/v1/crm/deals/{deal_id}`
  - Chi tiết deal. [file:2]

- `PATCH /api/v1/crm/deals/{deal_id}`
  - Cập nhật deal. [file:2]

- `POST /api/v1/crm/deals/{deal_id}/move-stage`
  - Đổi stage pipeline (NEW → QUALIFIED → PROPOSAL → WON/LOST...). [file:2]

### 5.3. Activities

- `GET /api/v1/crm/activities`
  - Danh sách activity (call/email/meeting/task). [file:2]

- `POST /api/v1/crm/activities`
  - Tạo activity mới. [file:2]

- `GET /api/v1/crm/activities/{activity_id}`
  - Chi tiết activity. [file:2]

---

## 6. Nhóm API HRM (hrm-svc)

**Base path**: `/api/v1/hrm` [file:2][file:6]

### 6.1. Employees

- `GET /api/v1/hrm/employees`
  - Danh sách nhân sự. [file:2]

- `POST /api/v1/hrm/employees`
  - Tạo hồ sơ nhân sự. [file:2]

- `GET /api/v1/hrm/employees/{employee_id}`
  - Chi tiết nhân sự. [file:2]

- `PATCH /api/v1/hrm/employees/{employee_id}`
  - Cập nhật hồ sơ. [file:2]

### 6.2. Attendance (chấm công)

- `GET /api/v1/hrm/attendances`
  - Danh sách bản ghi chấm công (filter theo employee, date...). [file:2]

- `POST /api/v1/hrm/attendances`
  - Ghi nhận chấm công (check-in/check-out). [file:2]

### 6.3. Leaves (nghỉ phép)

- `GET /api/v1/hrm/leaves`
  - Danh sách đơn nghỉ. [file:2]

- `POST /api/v1/hrm/leaves`
  - Tạo đơn nghỉ. [file:2]

- `PATCH /api/v1/hrm/leaves/{leave_id}`
  - Cập nhật/trạng thái duyệt/không duyệt. [file:2]

---

## 7. Nhóm API Reporting (reporting-svc)

**Base path**: `/api/v1/reporting` [file:2][file:6]

### 7.1. Dashboard & Summary

- `GET /api/v1/reporting/overview`
  - Tổng quan KPI chính (doanh thu, đơn hàng, tồn kho, HRM, CRM...). [file:2][file:6]

- `GET /api/v1/reporting/sales-summary`
  - Tổng hợp doanh số theo ngày/tuần/tháng. [file:2]

- `GET /api/v1/reporting/inventory-summary`
  - Tổng hợp tồn kho. [file:2]

- `GET /api/v1/reporting/crm-funnel`
  - Funnel CRM (số deal từng stage). [file:2]

- `GET /api/v1/reporting/hrm-summary`
  - Tổng quan HRM (headcount, attendance, v.v.). [file:2]

### 7.2. Reports chi tiết & export

- `GET /api/v1/reporting/orders`
  - Báo cáo chi tiết đơn hàng (có filter/pagination). [file:2]

- `GET /api/v1/reporting/customers`
  - Báo cáo chi tiết khách hàng. [file:2]

- `GET /api/v1/reporting/employees`
  - Báo cáo nhân sự. [file:2]

- `GET /api/v1/reporting/export`
  - Endpoint chung để export (CSV/Excel/PDF) theo loại report. [file:2]

---

## 8. Nhóm API AI (ai-svc)

**Base path**: `/api/v1/ai` [file:2][file:6]

### 8.1. Query & Assistant

- `POST /api/v1/ai/query`
  - Endpoint tổng quát để AI trả lời câu hỏi dựa trên dữ liệu nền tảng (RAG + logic nghiệp vụ). [file:2][file:6]

- `POST /api/v1/ai/insight`
  - Yêu cầu AI tạo insight (vd: phân tích doanh thu, tìm khách hàng rủi ro, gợi ý hành động). [file:2][file:6]

- `POST /api/v1/ai/assist`
  - Hỗ trợ theo ngữ cảnh màn hình nghiệp vụ (contextual assistant). [file:2][file:6]

### 8.2. Management (nội bộ/admin)

- `GET /api/v1/ai/models`
  - Danh sách model/config hiện có. [file:2][file:6]

- `POST /api/v1/ai/models/reload`
  - Reload cấu hình model/prompt (tuỳ chiến lược). [file:2][file:6]

---

## 9. Routing ở Gateway/Ingress

Tại API Gateway/Ingress (Nginx, Envoy, GKE Ingress, Kong, …), quy ước routing: [file:2][file:6]

- `/api/v1/auth/...`      → `auth-svc`
- `/api/v1/erp/...`       → `core-erp-svc`
- `/api/v1/crm/...`       → `crm-svc`
- `/api/v1/hrm/...`       → `hrm-svc`
- `/api/v1/reporting/...` → `reporting-svc`
- `/api/v1/ai/...`        → `ai-svc`

Có thể dùng:

- K8s Ingress + Service cho route basic.
- Hoặc API Gateway (Kong, Ambassador, Apigee, GCP API Gateway, …) nếu cần:
  - Rate limit,
  - Quota,
  - API key,
  - WAF (Web Application Firewall – tường lửa ứng dụng web). [file:2][file:1]

---

## 10. Hướng dẫn cho Dev & AI Agent

1. **Tuân thủ prefix & domain**  
   - Mọi route mới phải nằm dưới `/api/v1/<domain>/...` hợp lý. [file:2]

2. **Không trộn domain**  
   - Không đặt API CRM dưới `/erp` hay ngược lại. [file:2][file:6]

3. **Giữ resource RESTful**  
   - CRUD dùng `GET/POST/PATCH/DELETE` chuẩn. [file:2]
   - Action bổ sung dùng sub-path (vd: `/confirm`, `/complete`, `/cancel`) khi thực sự cần. [file:2]

4. **Cập nhật API-GATEWAY.md**  
   - Mỗi route mới thêm vào gateway phải được ghi lại tại đây, cùng:
     - Domain,
     - Mô tả ngắn,
     - Yêu cầu auth/permission (tham chiếu `SECURITY.md`). [file:2][file:1]

5. **AI Agent**  
   - Dùng file này như “bản đồ API”:
     - Biết route nào thuộc service nào,
     - Biết method/URL khi sinh client SDK, handler, test endpoint. [file:5][file:2]

---

## 11. Rule sử dụng API-GATEWAY.md cho AI Agent

- **AI Agent phải:**
  - Kiểm tra `API-GATEWAY.md` trước khi:
    - Thêm handler mới trong backend,
    - Sinh client call từ frontend/AI layer. [file:2][file:5]

- **AI Agent được:**
  - Sinh code handler tương ứng với route đã được định nghĩa ở đây.
  - Sinh SDK client cho các route. [file:5]

- **AI Agent không được:**
  - Tự tạo route `/api/v1/...` mới mà không:
    - Đề xuất/áp dụng cập nhật cho `API-GATEWAY.md`.
  - Đặt route sai domain (vd: API HRM dưới `/crm`). [file:2][file:6]