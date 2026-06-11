# SERVICE.md

## 1. Mục tiêu

Tài liệu này liệt kê **toàn bộ service** trong nền tảng Rust SaaS AI-First, giúp: [file:6][file:5]

- Dev & AI Agent nhìn nhanh:
  - Service nào làm gì,
  - Thuộc bounded context nào,
  - API chính & event chính của từng service.
- Làm “bản đồ entry point” dẫn tới các tài liệu chi tiết:
  - `ARCHITECTURE.md`, `AI-FIRST-ARCHITECTURE.md`, `API-GATEWAY.md`, `EVENTS.md`, `DATA-STRATEGY.md`. [file:6][file:5][file:2][file:7]

---

## 2. Danh sách service

| Service        | Domain chính                    | Trách nhiệm chính ngắn gọn                           |
|----------------|----------------------------------|------------------------------------------------------|
| auth-svc       | Auth, User, Tenant, RBAC        | Đăng nhập, JWT, user/tenant/role/permission         |
| core-erp-svc   | ERP (Order, Inventory, Invoice) | Đơn hàng, tồn kho, hoá đơn, thanh toán              |
| crm-svc        | CRM                             | Khách hàng, cơ hội, pipeline, activities            |
| hrm-svc        | HRM                             | Nhân sự, chấm công, nghỉ phép                       |
| reporting-svc  | Reporting / BI                  | Fact/dimension, dashboard, tổng hợp báo cáo         |
| ai-svc         | AI Kernel                       | AI query, insight, assistant, scoring, anomaly      |

Mỗi service có một file README riêng:

- `SERVICE-auth-svc.md`
- `SERVICE-core-erp-svc.md`
- `SERVICE-crm-svc.md`
- `SERVICE-hrm-svc.md`
- `SERVICE-reporting-svc.md`
- `SERVICE-ai-svc.md`

---

## 3. Liên kết tài liệu

- Kiến trúc tổng thể:
  - `ARCHITECTURE.md` [file:6]
- Kiến trúc code & AI-first:
  - `AI-FIRST-ARCHITECTURE.md` [file:5]
- API HTTP public:
  - `API-GATEWAY.md` [file:2]
- Chiến lược dữ liệu:
  - `DATA-STRATEGY.md` [file:7]
- Event & event bus:
  - `EVENTS.md`
- Auth flow & security:
  - `AUTH-FLOW.md`, `SECURITY.md` [file:3][file:1]
- Observability:
  - `OBSERVABILITY.md` [file:4]

---

## 4. Rule cho Dev & AI Agent

- Khi làm việc với **một service cụ thể**, hãy:
  1. Đọc `SERVICE-<service>.md`.
  2. Kết hợp với:
     - `ARCHITECTURE.md` (flow tổng) [file:6],
     - `AI-FIRST-ARCHITECTURE.md` (layout code) [file:5],
     - `API-GATEWAY.md` (route) [file:2],
     - `EVENTS.md` (event) [file:6],
     - `DATA-STRATEGY.md` (nơi lưu data) [file:7].