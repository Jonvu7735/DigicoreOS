# SERVICE-auth-svc.md

## 1. Mục tiêu & Domain

**Service**: `auth-svc` [file:6][file:5]

**Domain chính**:

- Authentication (xác thực),
- Authorization (phân quyền),
- RBAC (Role-Based Access Control – phân quyền theo vai trò),
- Multi-tenant (user, tenant). [file:1][file:3]

**Trách nhiệm**:

- Đăng nhập, đăng xuất, refresh token (JWT). [file:3]
- Quản lý user, tenant, role, permission. [file:1]
- Cấp JWT cho các client/service khác. [file:3][file:6]

---

## 2. API chính (qua API Gateway)

Xem chi tiết trong `API-GATEWAY.md`, nhóm `/api/v1/auth`. [file:2]

Các endpoint chính:

- `POST /api/v1/auth/login` – login. [file:3][file:2]
- `POST /api/v1/auth/refresh` – refresh access token. [file:3][file:2]
- `POST /api/v1/auth/logout` – logout. [file:3][file:2]
- `GET /api/v1/auth/me` – lấy thông tin user hiện tại. [file:2]
- Quản lý user:
  - `GET/POST/PATCH/DELETE /api/v1/auth/users/...` [file:2]
- Quản lý tenant:
  - `GET/POST/PATCH /api/v1/auth/tenants/...` [file:2]

---

## 3. Event publish/subscribe

Theo `ARCHITECTURE.md` & `EVENTS.md`: [file:6]

**Publish**

- `UserRegistered` → subject `platform.auth.user.registered`.
- `UserUpdated` → `platform.auth.user.updated`.
- `TenantCreated` → `platform.auth.tenant.created`.

**Subscribe**

- `EmployeeHired` (từ `hrm-svc`) → auto tạo user (nếu policy cho phép). [file:6]
- Các event khác liên quan đến identity (có thể mở rộng sau).

---

## 4. Data & Storage

- DB: PostgreSQL, schema: `auth_svc`. [file:5][file:7]
- Bảng chính:
  - `users`, `roles`, `permissions`, `user_roles`, `role_permissions`, `tenants`, `refresh_tokens`. [file:1][file:3]

Chi tiết chiến lược lưu trữ & multi-tenant → `DATA-STRATEGY.md`. [file:7]

---

## 5. Liên kết tài liệu

- Kiến trúc tổng:
  - `ARCHITECTURE.md` → mục 3.1 auth-svc. [file:6]
- Code layout & AI-first:
  - `AI-FIRST-ARCHITECTURE.md`. [file:5]
- API:
  - `API-GATEWAY.md` → nhóm `/api/v1/auth`. [file:2]
- Auth flow:
  - `AUTH-FLOW.md`. [file:3]
- Security & RBAC:
  - `SECURITY.md`. [file:1]
- Events:
  - `EVENTS.md` → nhóm Auth events.

---

## 6. Rule cho Dev & AI Agent

- Khi làm với `auth-svc`, phải đọc:
  - `SERVICE-auth-svc.md` (file này),
  - `AUTH-FLOW.md`, `SECURITY.md`, `API-GATEWAY.md`, `EVENTS.md`. [file:3][file:1][file:2]
- Không tự nhét logic nghiệp vụ khác (ERP/CRM/HRM) vào `auth-svc`.
- Mọi thay đổi JWT/RBAC phải cập nhật lại:
  - `AUTH-FLOW.md`, `SECURITY.md`, và file này.