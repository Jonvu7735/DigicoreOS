# SECURITY.md

## 1. Mục tiêu & Phạm vi

### Mục tiêu

Tài liệu này chuẩn hoá **chiến lược bảo mật** cho nền tảng Rust SaaS AI-First, tập trung vào: [file:1]

- Authentication (xác thực),
- Authorization (phân quyền),
- Mô hình RBAC (Role-Based Access Control – phân quyền theo vai trò),
- Bảo mật dữ liệu (data at rest, in transit),
- Quản lý secret, hardening (làm cứng) từng service. [file:1]

### Phạm vi

- Đây là **Single Source of Truth** cho:
  - RBAC model,
  - RBAC Matrix (role → permission),
  - Policy bảo mật (TLS, rate limit, secret, logging security). [file:1]
- Không mô tả:
  - Flow login/refresh/logout chi tiết → xem `AUTH-FLOW.md`. [file:3]
  - Danh sách route API → xem `API-GATEWAY.md`. [file:2]

---

## 2. Tổng quan bảo mật

### 2.1. Các lớp bảo mật

- Perimeter/Edge:
  - API Gateway/Ingress:
    - TLS termination,
    - Rate limiting,
    - IP filtering (nếu cần),
    - WAF (Web Application Firewall – tường lửa ứng dụng web). [file:1]
- Service:
  - Authn/Authz,
  - Validate input,
  - Kiểm soát quyền theo RBAC. [file:1][file:3]
- Data:
  - Mã hoá dữ liệu nhạy cảm (encryption),
  - Quản lý secret,
  - Backup an toàn. [file:1][file:7]

### 2.2. Authentication (Xác thực)

- Chuẩn khuyến nghị: [file:1][file:3]
  - JWT (JSON Web Token) dùng làm access token.
  - Refresh token:
    - Lưu an toàn trong HTTP-only cookie hoặc secure storage phía client.
- Token chứa các claim:
  - `sub`, `tenant_id`, `roles`, `exp`, `iss`, `aud` (xem `AUTH-FLOW.md`). [file:3]

### 2.3. Authorization (Phân quyền) với RBAC

- Mô hình RBAC:
  - User → Role → Permission. [file:1]

- Đặc điểm: [file:1]
  - User có thể thuộc nhiều role trong 1 tenant.
  - Role có nhiều permission.
  - Permission biểu diễn hành động trên resource (vd: `erp_order_read`, `erp_order_create`, `reporting_dashboard_view`). [file:1]

---

## 3. Mô hình dữ liệu bảo mật auth-svc

### 3.1. Thực thể chính

- `users`:
  - Người dùng hệ thống. [file:1]
- `roles`:
  - Vai trò (OWNER, ADMIN, MANAGER, STAFF, VIEWER…). [file:1]
- `permissions`:
  - Các quyền atomic (nhỏ nhất) theo cặp resource-action. [file:1]
- `role_permissions`:
  - Bảng many-to-many: role ↔ permission. [file:1]
- `user_roles`:
  - Bảng many-to-many: user ↔ role theo tenant. [file:1]

### 3.2. Context multi-tenant

- Một user có thể: [file:1][file:3]
  - Thuộc nhiều tenant,
  - Có role khác nhau ở mỗi tenant.
- Khi login:
  - Context gồm `tenant_id` + `roles` của user trong tenant đó. [file:3][file:1]

---

## 4. RBAC Matrix & Permission Strategy

### 4.1. Khái niệm

- Resource:
  - Thực thể nghiệp vụ (user, tenant, order, invoice, product, customer, deal, employee, report, ai-insight, …). [file:1][file:6]
- Action:
  - Hành động trên resource (read, create, update, delete, approve, …). [file:1]
- Permission:
  - Kết hợp `resource_action`, ví dụ:
    - `erp_order_read`,
    - `erp_order_create`,
    - `erp_order_cancel`,
    - `reporting_dashboard_view`. [file:1][file:3]

### 4.2. Nhóm resource chính

- Auth:
  - user, tenant, role, permission. [file:1]
- ERP:
  - order, invoice, product, inventory. [file:1][file:6]
- CRM:
  - customer, deal, activity. [file:1][file:6]
- HRM:
  - employee, attendance, leave. [file:1][file:6]
- Reporting:
  - dashboard, report, export. [file:1][file:7]
- AI:
  - assistant use, ai config. [file:1][file:6]

### 4.3. RBAC Matrix (phiên bản đơn giản)

*(Đây là ví dụ, bạn có thể mở rộng trong DB.)* [file:1]

**Nhãn role:**

- OWNER: chủ doanh nghiệp (toàn quyền trong tenant).
- ADMIN: quản trị hệ thống trong tenant.
- MANAGER: quản lý bộ phận (sales, ops, …).
- STAFF: nhân viên thao tác nghiệp vụ.
- VIEWER: chỉ xem.

**Auth & Tenant**

- `auth_user_read`
- `auth_user_create`
- `auth_user_update`
- `auth_user_assign_role`
- `auth_tenant_read`
- `auth_tenant_update_plan`

**ERP (Orders, Invoices, Products, Inventory)**

- `erp_order_read`
- `erp_order_create`
- `erp_order_update`
- `erp_order_cancel`
- `erp_invoice_read`
- `erp_invoice_create`
- `erp_invoice_cancel`
- `erp_product_read`
- `erp_product_create`
- `erp_product_update`

**CRM**

- `crm_customer_read`
- `crm_customer_create`
- `crm_customer_update`
- `crm_deal_read`
- `crm_deal_create`
- `crm_deal_update`
- `crm_deal_move_stage`

**HRM**

- `hrm_employee_read`
- `hrm_employee_create`
- `hrm_employee_update`
- `hrm_attendance_read`
- `hrm_attendance_create`

**Reporting & AI**

- `reporting_dashboard_view`
- `reporting_report_export`
- `ai_assistant_use`
- `ai_config_manage`

Chi tiết mapping role → permission nên được lưu trong DB (bảng `role_permissions`). `SECURITY.md` dùng để mô tả logic & guideline. [file:1]

### 4.4. Cách implement permission check

- Tại mỗi service: [file:1][file:3][file:5]
  - Extract `user_id`, `tenant_id`, `roles` từ JWT/context.
  - Lấy (hoặc cache) mapping role → permission (từ `auth-svc` hoặc DB).
  - Kiểm tra:
    - `can(user).do(permission)`? (vd: `can(user).do("erp_order_create")`).
- Hai lớp:
  - Coarse-grained:
    - Rule đơn giản (VD: chỉ OWNER/ADMIN được gọi API nhạy cảm).
  - Fine-grained:
    - Check permission cụ thể theo resource/action.

---

## 5. Bảo mật API Gateway

### 5.1. TLS & Network

- Tất cả traffic từ client đến gateway:
  - Phải dùng HTTPS (TLS). [file:1]
- Traffic nội bộ giữa service:
  - Tối thiểu giới hạn bằng network policy (K8s, VPC).
  - Có thể dùng mTLS (mutual TLS – TLS 2 chiều) cho môi trường yêu cầu cao. [file:1]

### 5.2. Rate limiting & Throttling

- Tại gateway:
  - Rate limit theo IP hoặc API key (khi có public API). [file:1][file:2]
  - Ưu tiên bảo vệ endpoint nhạy cảm:
    - `/auth/login`,
    - `/auth/refresh`. [file:1][file:3]

### 5.3. Input validation & Sanitization

- Mọi input từ client:
  - Phải được validate kiểu dữ liệu, độ dài, range. [file:1][file:5]
  - Sanitization:
    - Tránh injection (SQL, NoSQL, command injection). [file:1]

---

## 6. Bảo mật dữ liệu & Secret

### 6.1. Data at rest (dữ liệu khi lưu trữ)

- Database:
  - Encryption at rest (tính năng của cloud provider hoặc layer DB). [file:1][file:7]
- Backup:
  - Backup mã hoá,
  - Quản lý key an toàn (KMS). [file:1][file:7]

### 6.2. Data in transit (dữ liệu khi truyền)

- HTTPS/TLS cho mọi traffic public. [file:1]
- mTLS cho traffic nội bộ (tuỳ yêu cầu). [file:1]

### 6.3. Secret management

- Không commit secret vào git. [file:1]
- Sử dụng:
  - Secret manager (GCP Secret Manager, AWS Secrets Manager, Vault, …),
  - Hoặc K8s Secret kết hợp KMS giai đoạn đầu. [file:1][file:6]

---

## 7. Logging security

- Logging bảo mật nên: [file:1][file:4]
  - Ghi nhận hành động nhạy cảm:
    - login, logout,
    - đổi mật khẩu,
    - đổi role,
    - tạo/xoá user,
    - đổi quyền, đổi plan.
  - Ghi nhận hành vi bất thường:
    - nhiều lần login fail,
    - nhiều truy cập bị deny. [file:1][file:4]

- Không log:
  - Mật khẩu,
  - Token (access/refresh),
  - Dữ liệu nhạy cảm (PII chưa được mask). [file:1][file:4]

Các log này hỗ trợ:

- Audit,
- Điều tra sự cố bảo mật,
- Huấn luyện AI Agent phát hiện hành vi bất thường (AI Ops/AI Security). [file:1][file:4][file:7]

---

## 8. Rule cho Dev & AI Agent

### 8.1. Khi thêm API mới

- Phải xác định rõ:
  - Resource,
  - Action,
  - Permission name tương ứng. [file:1][file:2]
- Cập nhật:
  - Bảng RBAC trong DB,
  - Nếu cần, mô tả ở `SECURITY.md` (phần permission mới). [file:1]
- Áp dụng check permission tại handler/middleware. [file:3][file:5]

### 8.2. Khi thêm role mới

- Xác định rõ vai trò & phạm vi role (scope). [file:1]
- Gán permission phù hợp trong RBAC Matrix & DB.
- Tránh tạo quá nhiều role khó quản lý.

### 8.3. AI Agent

- **AI Agent phải**:
  - Dùng `SECURITY.md` làm nguồn chuẩn:
    - RBAC model,
    - Permission naming. [file:1][file:5]
- **AI Agent được**:
  - Sinh:
    - Middleware check permission,
    - Migration bảng role/permission/user_role,
    - Test cho các case thiếu quyền. [file:1][file:5]
- **AI Agent không được**:
  - Tự sửa RBAC Matrix (role → permission) mà không có chỉ thị cụ thể từ con người.
  - Thay đổi policy bảo mật (TLS, secret, logging nhạy cảm) mà không cập nhật doc này.