# AUTH-FLOW.md

## 1. Mục tiêu & Phạm vi

### Mục tiêu

Tài liệu này mô tả **luồng xác thực (Authentication)** và **phân quyền runtime (Authorization)** end-to-end cho nền tảng Rust SaaS AI-First, bao gồm: [file:3]

- Login, refresh token, logout.
- Cách API Gateway và các service backend kiểm tra JWT & RBAC (Role-Based Access Control – phân quyền theo vai trò). [file:3][file:1]
- Cách AI Agent hiểu và sinh code đúng với thiết kế này. [file:3][file:5]

### Phạm vi

- Tập trung vào **flow runtime** và JWT:
  - Cấu trúc token,
  - Cách validate/check permission. [file:3]
- Không chứa:
  - Ma trận RBAC chi tiết (role → permission) → xem `SECURITY.md`. [file:1]
  - Danh sách route API → xem `API-GATEWAY.md`. [file:2]

---

## 2. Thành phần tham gia

- Client:
  - Frontend Next.js, mobile app, hoặc API client khác. [file:3][file:6]
- API Gateway/Ingress:
  - Điểm vào HTTP duy nhất của backend, có thể thực hiện:
    - TLS termination,
    - Một phần auth coarse-grained. [file:3][file:1]
- auth-svc:
  - Service chịu trách nhiệm:
    - login, refresh, logout,
    - quản lý user/tenant/role/permission. [file:3][file:1][file:6]
- Các service domain:
  - `core-erp-svc`, `crm-svc`, `hrm-svc`, `reporting-svc`, `ai-svc`. [file:3][file:6]

**Token context**

- Access token:
  - JWT, thời gian sống ngắn (short-lived), dùng cho mỗi request protected. [file:3][file:1]
- Refresh token:
  - Token thời gian sống dài hơn (long-lived), dùng để lấy access token mới.
  - Được lưu an toàn phía client:
    - HTTP-only cookie (web),
    - secure storage (mobile). [file:3]

---

## 3. Cấu trúc JWT

Ví dụ JWT header & payload (mã hoá JSON Web Token – chuẩn RFC 7519): [file:3][file:1]

```json
{
  "alg": "RS256",
  "typ": "JWT"
}
```

```json
{
  "sub": "user_id",
  "tenant_id": "tenant_id",
  "roles": ["OWNER", "ADMIN"],
  "iat": 1710000000,
  "exp": 1710003600,
  "iss": "auth-svc",
  "aud": "platform-api"
}
```

Giải thích claim chính:

- `sub`:
  - Subject – ID người dùng. [file:3]
- `tenant_id`:
  - Tenant hiện đang active trong session. [file:3][file:1]
- `roles`:
  - Danh sách role của user trong tenant hiện tại. [file:3][file:1]
- `exp`:
  - Thời điểm hết hạn access token (epoch seconds). [file:3]
- `iss`:
  - Issuer – service phát token (`auth-svc`). [file:3]
- `aud`:
  - Audience – đối tượng token được phát cho (`platform-api`). [file:3]

**Chi tiết model RBAC (role/permission)** xem trong `SECURITY.md`. [file:1]

---

## 4. Luồng Login

### 4.1. Mục tiêu

- Xác thực user (email/password hoặc SSO),
- Trả về access token và refresh token,
- Ghi log bảo mật phù hợp. [file:3][file:1]

### 4.2. Các bước

1. **Client → Gateway** [file:3][file:2]

   - HTTP request:

   ```http
   POST /api/v1/auth/login
   Content-Type: application/json

   {
     "email": "user@example.com",
     "password": "...",
     "tenant_id": "optional_if_multi_tenant"
   }
   ```

2. **Gateway → auth-svc**

   - Gateway route `/api/v1/auth/login` đến `auth-svc`. [file:2][file:6]

3. **Xử lý tại auth-svc**

   - Validate input.
   - Tìm user theo email.
   - Verify password (hash + salt, ví dụ Argon2/bcrypt). [file:3]
   - Nếu user thuộc nhiều tenant:
     - Xác định `tenant_id` context (dựa vào input hoặc lựa chọn sau). [file:3][file:1]
   - Lấy danh sách roles của user trong tenant đó.
   - Sinh access token (JWT) với claim:
     - `sub`, `tenant_id`, `roles`, `exp` (ngắn, VD: 15–30 phút), `iss`, `aud`. [file:3][file:1]
   - Sinh refresh token:
     - TTL dài hơn (VD: 7–30 ngày),
     - Lưu server-side (DB) hoặc dùng opaque token có thể revoke. [file:3][file:1]

4. **auth-svc → Client**

   - Response JSON: [file:3]

   ```json
   {
     "access_token": "<jwt>",
     "token_type": "Bearer",
     "expires_in": 1800,
     "refresh_token": "<refresh_token>",
     "user": {
       "id": "user_id",
       "email": "user@example.com",
       "display_name": "...",
       "tenant_id": "tenant_id",
       "roles": ["ADMIN"]
     }
   }
   ```

   - Frontend:
     - Lưu `access_token` trong memory/secure storage.
     - Lưu `refresh_token` trong HTTP-only cookie (web) hoặc secure storage (mobile). [file:3]

5. **Logging & Metrics**

   - Log:
     - `INFO` cho login success/fail (không log password). [file:3][file:4]
   - Metrics:
     - `auth_login_success_total`
     - `auth_login_failed_total` [file:3][file:4]

---

## 5. Luồng Refresh Token

### 5.1. Mục tiêu

- Cấp access token mới khi token cũ gần hết hạn hoặc đã hết hạn,
- Không bắt user đăng nhập lại, trừ khi refresh token không hợp lệ. [file:3]

### 5.2. Các bước

1. **Client → Gateway**

   ```http
   POST /api/v1/auth/refresh
   Content-Type: application/json

   {
     "refresh_token": "<refresh_token>"
   }
   ```

   Hoặc gửi refresh token trong cookie (HTTP-only). [file:3]

2. **Gateway → auth-svc**

   - Route đến `auth-svc`. [file:2][file:6]

3. **Xử lý tại auth-svc**

   - Validate refresh token:
     - Tồn tại,
     - Chưa hết hạn,
     - Chưa bị revoke. [file:3][file:1]
   - Lấy `user_id`, `tenant_id`, `roles` tương ứng.
   - Sinh access token mới (JWT).
   - Option:
     - Rotate refresh token (cấp token mới, revoke token cũ). [file:3]

4. **Response**

   ```json
   {
     "access_token": "<new_jwt>",
     "token_type": "Bearer",
     "expires_in": 1800,
     "refresh_token": "<new_refresh_token_optional>"
   }
   ```

5. **Logging & Metrics**

   - Log INFOWARN cho refresh thành công/thất bại. [file:3][file:4]
   - Metrics:
     - `auth_refresh_success_total`
     - `auth_refresh_failed_total` [file:3][file:4]

---

## 6. Luồng Logout

### 6.1. Mục tiêu

- Kết thúc session,
- Đảm bảo refresh token không dùng lại được. [file:3][file:1]

### 6.2. Các bước

1. **Client → Gateway**

   ```http
   POST /api/v1/auth/logout
   Authorization: Bearer <access_token>

   {
     "refresh_token": "<refresh_token>"
   }
   ```

2. **Gateway → auth-svc**

   - Forward request. [file:2][file:6]

3. **auth-svc xử lý**

   - Xác thực access token (optional nhưng nên có để log user). [file:3]
   - Revoke refresh token trong DB. [file:3][file:1]

4. **Client**

   - Xoá access token & refresh token ở phía client (memory/cookie). [file:3]

---

## 7. Kiểm tra JWT & RBAC tại Gateway & Service

### 7.1. Tại API Gateway

- Flow cho route protected `/api/v1/...`: [file:3][file:2][file:1]

  1. Lấy header `Authorization: Bearer <jwt>`.
  2. Verify chữ ký JWT (RS256/HS256) bằng public key/secret. [file:3][file:1]
  3. Kiểm tra `exp`, `iss`, `aud`.
  4. Nếu fail → trả `401 Unauthorized`.
  5. Nếu pass:
     - Có thể:
       - Thêm thông tin user vào header (X-User-Id, X-Tenant-Id, X-Roles),
       - Hoặc giữ nguyên JWT forward xuống service. [file:3]

- Optional:
  - Gateway có thể có rule coarse-grained, ví dụ:
    - Block toàn bộ request từ user bị global-block. [file:3][file:1]

### 7.2. Tại service domain (ERP/CRM/HRM/Reporting/AI)

- Mỗi service có middleware: [file:3][file:5]

  1. Parse & verify JWT (có thể tin vào gateway và chỉ verify lại nếu cần).
  2. Extract context:
     - `user_id`, `tenant_id`, `roles`. [file:3]
  3. Map roles → permissions:
     - Cache local mapping role-permission (sync từ `auth-svc`/DB),
     - Mô hình permission xem `SECURITY.md`. [file:1]
  4. Check permission theo route/action:
     - Ví dụ:
       - `POST /api/v1/erp/orders` → cần permission `erp_order_create`,
       - `GET /api/v1/crm/customers` → `crm_customer_read`,
       - `GET /api/v1/reporting/overview` → `reporting_dashboard_view`. [file:3][file:1][file:2]
  5. Nếu thiếu quyền → trả `403 Forbidden`.

### 7.3. Mapping API → permission

- Mapping chi tiết (route → permission) nên:
  - Định nghĩa trong `SECURITY.md` (RBAC Matrix) và/hoặc bảng mapping trong DB. [file:1]
  - `AUTH-FLOW.md` chỉ cần nêu ví dụ để minh hoạ. [file:3]

---

## 8. Observability cho Auth Flow

- Logging:
  - Log login success/fail, refresh success/fail, logout. [file:3][file:4]
  - Không log password/token. [file:1][file:4]
- Metrics:
  - `auth_login_success_total`, `auth_login_failed_total`.
  - `auth_refresh_success_total`, `auth_refresh_failed_total`.
  - `auth_active_sessions` (nếu có mô hình session). [file:3][file:4]
- Traces:
  - Trace toàn bộ flow client → gateway → auth-svc → DB. [file:3][file:4]
  - Thêm span cho:
    - verify password,
    - generate token. [file:3][file:4]

---

## 9. Rule sử dụng AUTH-FLOW.md cho AI Agent

- **Trước khi sinh code liên quan auth**, AI Agent phải đọc: [file:3][file:1][file:2]
  - `AUTH-FLOW.md` – flow login/refresh/logout, JWT.
  - `SECURITY.md` – RBAC model & permission.
  - `API-GATEWAY.md` – route auth.

- **AI Agent được:**
  - Sinh handler login/refresh/logout trong `auth-svc`.
  - Sinh middleware verify JWT & check permission cho các service domain.
  - Sinh test cho các scenario:
    - token hết hạn,
    - refresh token invalid,
    - thiếu quyền. [file:3][file:5]

- **AI Agent không được:**
  - Tự sửa cấu trúc JWT (claim `sub`, `tenant_id`, `roles`, `exp`, `iss`, `aud`) nếu không có yêu cầu rõ. [file:3][file:1]
  - Tự thay đổi policy RBAC (role → permission) – nguồn chuẩn là `SECURITY.md`. [file:1]