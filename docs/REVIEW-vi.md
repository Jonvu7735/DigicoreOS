# Báo cáo Review Codebase — DigicoreOS

> Phạm vi: toàn bộ monorepo (6 microservices core + 2 verticals + frontend + libs + CI).
> Ngày: 2026-06-13. Nhánh review: `claude/vietnamese-review-reporting-6aj8jr`.
> Hình thức: đánh giá tĩnh (đọc code + tài liệu), không chạy hệ thống. Mọi nhận định
> đều trích dẫn `file:line` để kiểm chứng trực tiếp.

---

## 1. Tóm tắt điều hành

DigicoreOS là một **codebase trưởng thành và kỷ luật cao**. Kiến trúc hexagonal (domain /
ports / infra) được áp dụng nhất quán, có transactional outbox, schema-per-service, RBAC
tập trung, và một hệ thống CI nhiều tầng rất chặt chẽ (clippy `-D warnings`, cargo-deny,
kiểm tra hợp đồng OpenAPI, đồng bộ client TypeScript, validate docker-compose). Tài liệu
trong `docs/` phong phú và bám sát code.

Các vấn đề tìm thấy tập trung vào **độ bền (durability) của event bus** và một số điểm
reliability, **không phải lỗi cấu trúc nền tảng**. Đánh giá tổng thể: **Tốt**, có thể đưa
vào production sau khi xử lý 2 phát hiện mức CAO bên dưới.

| Mức | Số lượng | Vấn đề chính |
|-----|----------|--------------|
| 🔴 Cao | 2 | Event bus dùng core NATS (mất sự kiện); consumer nuốt lỗi không retry/DLQ |
| 🟠 Trung bình | 3 | LLM client không timeout; relay không khóa hàng khi scale; prompt injection nhẹ |
| 🟡 Thấp | 2 | Doc drift (`platform-ratelimit`); con số `unwrap` dễ gây hiểu nhầm |

**Ưu tiên hành động:** (1) chuyển event bus sang JetStream hoặc bổ sung cơ chế replay từ
outbox; (2) thêm retry/DLQ cho consumer; (3) thêm timeout cho HTTP client gọi LLM.

---

## 2. Tổng quan kiến trúc & mức độ tuân thủ nguyên tắc

**Mô hình:** Rust workspace, 6 microservice core trên Postgres dùng chung (schema-per-service)
và NATS event bus. Mỗi write tạo event được commit chung state + event trong một transaction
(transactional outbox). Analytics & AI được xây bằng cách **consume** các event đó.

**Phân tầng (mỗi service):** `domain/` (entities, ports, services) → `infra/` (db, security,
time, ai) → `api/http/` (handlers, dto, middleware, routes) → `bootstrap/` (config, wiring).
Đây là hexagonal/ports-and-adapters chuẩn mực; ví dụ `auth-svc` tách rõ
`domain/identity/ports.rs` khỏi `infra/db/*_repo_pg.rs` và `infra/security/{jwt,password}.rs`.

**Mức độ tuân thủ "Architecture rules":**
- ✅ **Schema-per-service**: mỗi service có schema riêng, migrations riêng.
- ✅ **Transactional outbox**: state + event ghi trong một transaction qua
  `platform_outbox::insert_outbox` (`libs/platform-outbox/src/pg.rs:18`), được gọi bên trong
  `tx` của repository (ví dụ `services/crm-svc/.../deals_repo_pg.rs`).
- ✅ **Verticals chỉ phụ thuộc API/event công khai** (`verticals/README.md:1`): retail-svc và
  trade-export-svc là workspace độc lập, không import crate core.
- ✅ **Ports pattern**: các trait port (`OutboxRepository`, `RawEventPublisher`,
  `InboundEventHandler`, `Assistant`, `PasswordHasher`) cho phép thay adapter (thật ↔ stub/fake).

**Drift nhỏ:** `libs/platform-ratelimit` đã được wire vào routes của mọi service nhưng phần
"Layout" trong `README.md` không liệt kê nó (xem mục 9, finding #6).

---

## 3. Bảo mật & cô lập đa tenant

**Xác thực (Authentication):**
- JWT **RS256**, cố định thuật toán (chống alg-confusion) và bắt buộc kiểm `iss` + `aud`:
  `libs/platform-auth/src/verify.rs:28-30`. Verifier chỉ giữ public key; việc phát hành token
  nằm ở `auth-svc`. Hết hạn (`exp`) được `jsonwebtoken` kiểm mặc định.
- Mật khẩu băm bằng **Argon2** với salt ngẫu nhiên từ `OsRng`
  (`services/auth-svc/src/infra/security/password.rs`); không log/lưu mật khẩu thô.

**Phân quyền (Authorization):** RBAC matrix tập trung trong `libs/platform-auth/src/rbac.rs`
là nguồn sự thật duy nhất cho role→permission (5 role mặc định + 42 permission), được mọi
service enforce thống nhất.

**Cô lập đa tenant:** ✅ Tốt. Rà soát các câu `SELECT` ở core services (erp/crm/hrm) **không
phát hiện truy vấn nào thiếu lọc `tenant_id`**. `tenant_id` đến từ claim trong access token,
không phải từ input người dùng → giảm rủi ro truy cập chéo tenant.

**An toàn SQL:** ✅ Toàn bộ truy vấn dùng `sqlx` tham số hóa (`.bind(...)`); không thấy SQL
nối chuỗi → không có bề mặt SQL injection.

**Rate limiting:** `libs/platform-ratelimit` được áp vào routes của tất cả service.

**LLM/secret:** API key lấy từ env `ANTHROPIC_API_KEY` (`bootstrap/config.rs`), **không có
default commit trong repo**; nếu thiếu key/model thì tự fallback sang stub. Key không bị log.

---

## 4. Đúng đắn của transactional outbox / event bus

**Phía Producer (outbox) — vững:**
- `insert_outbox` ghi event cùng transaction với state (`pg.rs:18`).
- Relay đọc theo thứ tự `ORDER BY created_at` (`pg.rs:84`), publish rồi mới `mark_published`
  (`relay.rs:45-47`) → mô hình **publish-then-mark = at-least-once** phía producer. Dừng ở
  lỗi publish đầu tiên để giữ thứ tự và retry ở tick sau (`relay.rs:38`).

**Phía Consumer — idempotent:** read-model dedupe bằng `ON CONFLICT (event_id) DO NOTHING`
(ví dụ `services/reporting-svc/src/infra/db/sales_repo_pg.rs:41`,
`inventory_repo_pg.rs:40`) → re-delivery cùng event là no-op. Tốt.

### 🔴 Finding #1 (CAO) — Event bus dùng core NATS, không JetStream
`libs/platform-events/src/consumer.rs:27` dùng `client.subscribe("platform.>")` thuần (core
NATS, ephemeral). Trong khi đó relay gọi `mark_published` ngay khi NATS nhận message
(`relay.rs:45-47`). Vấn đề: **core NATS trả `Ok` cho `publish` kể cả khi không có subscriber
nào đang kết nối — và message bị bỏ.** Hệ quả: nếu reporting-svc/ai-svc offline đúng lúc relay
publish, sự kiện **mất vĩnh viễn** nhưng outbox đã đánh dấu `published_at` (sẽ không bao giờ
gửi lại). Comment "at-least-once" trong `relay.rs` chỉ đúng phía producer, **không đúng
end-to-end**.

> **Khuyến nghị:** Chuyển sang **NATS JetStream** (stream bền + durable consumer + `ack`), hoặc
> giữ core NATS nhưng bổ sung cơ chế replay: consumer lưu offset/checkpoint và yêu cầu relay
> gửi lại theo `created_at` khi khởi động. Lựa chọn JetStream là tự nhiên nhất với kiến trúc
> hiện tại.

### 🔴 Finding #2 (CAO) — Consumer nuốt lỗi handler, không retry/DLQ
`consumer.rs:45`: khi `handler.handle(...)` trả `Err`, code chỉ `tracing::warn!` rồi bỏ qua
message. Kết hợp với core NATS (Finding #1), một lỗi DB **tạm thời** khi xử lý event = mất
event đó vĩnh viễn (không có lần giao lại).

> **Khuyến nghị:** Với JetStream, dùng `nak`/không-ack để được giao lại + cấu hình max-deliver
> và **Dead-Letter Queue** cho message lỗi vĩnh viễn. Tối thiểu: thêm retry có backoff cho lỗi
> tạm thời trước khi bỏ.

### 🟠 Finding #4 (TRUNG BÌNH) — Relay không khóa hàng khi scale-out
`fetch_unpublished` (`pg.rs:80`) chỉ `SELECT ... WHERE published_at IS NULL ORDER BY
created_at LIMIT $1`, không có `FOR UPDATE SKIP LOCKED`. Nếu chạy nhiều instance service, hai
relay có thể đọc cùng batch → **publish trùng** (được giảm nhẹ nhờ consumer idempotent, nhưng
tốn tài nguyên và có thể đảo thứ tự).

> **Khuyến nghị:** Thêm `FOR UPDATE SKIP LOCKED` khi đọc batch, hoặc chỉ định một instance làm
> relay (leader election) nếu không có ý định scale relay theo chiều ngang.

---

## 5. Chất lượng code & xử lý lỗi

- **Mô hình lỗi:** domain dùng `DomainError`/`DomainResult` + `thiserror`; HTTP handler map
  lỗi sang status phù hợp qua DTO `error.rs` ở mỗi service. Sạch và nhất quán.
- **`unwrap`/`expect`/`panic` — phần lớn vô hại (🟡 Finding #7):** grep thô cho ra số lớn (vd
  reporting-svc 120 occurrence), nhưng đa số nằm trong **test** và các `expect` **bất khả lỗi**
  như `and_hms_opt(0,0,0).expect("midnight is valid")`
  (`reporting-svc/src/api/http/dto/date_range.rs:35`). Không phải rủi ro panic thực sự trên
  đường xử lý request. Nêu ở đây để con số grep không bị hiểu nhầm.
- **Không có `TODO`/`FIXME`** trong code Rust → ít nợ kỹ thuật được đánh dấu treo.
- **DB pool:** có `acquire_timeout(3s)` (`*/infra/db/postgres.rs`) — hợp lý cho K8s probe.

---

## 6. Kiểm thử & CI/CD

**CI (`.github/workflows/ci.yml`) — rất đầy đủ:**
- `check`: `cargo fmt --check`, `clippy --all-targets -D warnings`, `cargo test`.
- `integration`: chạy test với **Postgres 16 + NATS 2.10** thật (`TEST_DATABASE_URL`,
  `TEST_NATS_URL`), gồm E2E backbone sự kiện (`reporting-svc/e2e.rs`).
- `supply-chain`: **cargo-deny** (advisories, bans, sources).
- `vertical-trade-export` / `vertical-retail`: build + test các vertical workspace riêng.
- `openapi-contract`: mọi route `/api/v1` được phục vụ phải có trong `docs/openapi.yaml`.
- `openapi-client` / `frontend`: client TS và schema FE phải đồng bộ với `openapi.yaml`
  (fail nếu stale).
- `compose`: validate `docker-compose.dev.yml` đầy đủ stack.

**Test (51 file có test):** đủ tầng unit (logic domain với fake), integration (DB-gated), và
E2E qua NATS. Outbox relay có test cho cả happy-path lẫn failure (`relay.rs:119,135`); JWT có
round-trip RS256 + reject sai key/garbage (`verify.rs`). Chất lượng test tốt.

> Gợi ý: bổ sung test mô phỏng kịch bản consumer offline để bộc lộ Finding #1/#2.

---

## 7. Frontend

- **Stack:** React 19 + React Router 7 + Vite 8 + TypeScript 5.6, gọi API qua
  `openapi-fetch` với schema sinh tự động (`frontend/src/api/schema.d.ts`) — **type-safe
  end-to-end** với hợp đồng OpenAPI, được CI canh đồng bộ.
- **Cấu trúc:** `auth/` (AuthProvider + context), `components/ProtectedRoute.tsx`, các trang
  Login/Home/Demo/Assistant/Loyalty/Shipments. Gọn gàng, phân tách rõ.
- CI chạy `lint` + `typecheck`/`build`, nên rủi ro type/lint thấp.

> Gợi ý (chưa kiểm sâu): rà soát trạng thái loading/error tường minh trên các trang gọi API
> (đặc biệt AssistantPage gọi LLM có thể chậm) để UX không bị "treo".

---

## 8. Verticals

- `retail-svc` (loyalty points: ledger earn/redeem, loyalty rules theo tenant) và
  `trade-export-svc` (vòng đời shipment: dispatch/cancel, cargo lines, timeline) là **workspace
  độc lập**, có migrations riêng, tuân thủ đúng quy tắc "core via public APIs/events only"
  (`verticals/README.md`). Không import crate core. ✅
- Cùng kiến trúc hexagonal như core, có CI gate riêng (fmt/clippy/test với Postgres).

---

## 9. Bảng phát hiện theo mức ưu tiên

| # | Mức | Vấn đề | Vị trí | Khuyến nghị |
|---|-----|--------|--------|-------------|
| 1 | 🔴 Cao | Core NATS (ephemeral) → mất event nếu consumer offline, dù outbox đã mark published | `libs/platform-events/src/consumer.rs:27`, `libs/platform-outbox/src/relay.rs:45-47` | Dùng JetStream (durable + ack) hoặc replay từ outbox theo `created_at` |
| 2 | 🔴 Cao | Consumer nuốt lỗi handler, không retry/DLQ → lỗi tạm thời = mất event | `libs/platform-events/src/consumer.rs:45` | `nak`/retry-backoff + Dead-Letter Queue |
| 3 | 🟠 TB | HTTP client gọi LLM không có timeout → request có thể treo vô hạn | `services/ai-svc/src/infra/ai/claude_assistant.rs:38` | `reqwest::Client::builder().timeout(...)` + retry/backoff |
| 4 | 🟠 TB | Relay không `FOR UPDATE SKIP LOCKED` → publish trùng khi scale-out | `libs/platform-outbox/src/pg.rs:80-84` | Thêm SKIP LOCKED hoặc leader election |
| 5 | 🟠 TB | Prompt injection nhẹ: `context` của caller nối thẳng vào prompt | `services/ai-svc/src/infra/ai/claude_assistant.rs:84` | Giới hạn/đánh dấu rõ ranh giới context; ghi chú rủi ro |
| 6 | 🟡 Thấp | `platform-ratelimit` không xuất hiện trong "Layout" của README | `README.md`, `libs/platform-ratelimit/` | Bổ sung vào danh sách libs |
| 7 | 🟡 Thấp | Con số `unwrap`/`expect` lớn nhưng đa phần trong test/bất khả lỗi | nhiều file | Không cần sửa; lưu ý khi đọc kết quả grep |

---

## 10. Khuyến nghị & lộ trình đề xuất

**Ngắn hạn (trước production):**
1. Khắc phục Finding #1 + #2 (độ bền event bus) — đây là rủi ro mất dữ liệu read-model/insight.
2. Thêm timeout (+ retry/backoff) cho HTTP client gọi LLM (Finding #3).

**Trung hạn (khi scale-out):**
3. `FOR UPDATE SKIP LOCKED` cho relay hoặc leader election (Finding #4).
4. Bổ sung test kịch bản consumer offline / poison message.

**Dọn dẹp nhỏ:**
5. Cập nhật README (Finding #6); ghi chú ranh giới context cho assistant (Finding #5).

**Kết luận:** Nền tảng kiến trúc, bảo mật và quy trình CI của DigicoreOS đều ở mức tốt đến rất
tốt. Rủi ro đáng kể duy nhất là **độ bền của lớp event bus** — xử lý xong là dự án sẵn sàng
vận hành tin cậy.

---

## 11. Trạng thái khắc phục (đã triển khai trong PR này)

Các phát hiện sau đã được vá ngay trong PR này (build + `clippy -D warnings` + `cargo fmt`
+ unit test đều pass cục bộ; đường E2E JetStream được CI kiểm trên NATS có bật `-js`):

| # | Trạng thái | Thay đổi |
|---|-----------|----------|
| 1 | ✅ Đã vá | Producer chuyển sang **JetStream**: `JetStreamPublisher` publish có server-ACK, chỉ `mark_published` sau khi event được lưu bền (`libs/platform-outbox/src/nats.rs`). Consumer dùng **durable consumer** (`libs/platform-events/src/consumer.rs`) → không còn mất event khi offline; stream tạo idempotent ở cả hai phía. |
| 2 | ✅ Đã vá | Consumer **ACK/NAK + DLQ**: thành công → ACK; lỗi tạm thời → NAK (backoff) để server giao lại; quá `MAX_DELIVER=5` → đẩy sang stream chết `platform_dlq` (`dlq.>`) rồi TERM. Không còn nuốt lỗi. |
| 3 | ✅ Đã vá | LLM HTTP client có `timeout(30s)` + `connect_timeout(5s)` + retry 3 lần cho lỗi transport tạm thời (`services/ai-svc/src/infra/ai/claude_assistant.rs`). |
| 4 | ✅ Đã vá (cách khác) | Thay vì `FOR UPDATE SKIP LOCKED`, dùng **dedup theo `Nats-Msg-Id` = event_id** + cửa sổ dedup 120s của JetStream: hai relay chạy song song / publish lặp lại đều bị server loại trùng — đúng đắn hơn và không phải giữ khóa DB qua mạng. |
| 6 | ✅ Đã vá | README "Layout" bổ sung `platform-ratelimit`. |
| 5 | ⏳ Ghi nhận | Prompt injection nhẹ: vẫn để nguyên hành vi (dữ liệu của chính tenant), đã ghi chú; chưa thay đổi logic. |
| 7 | ℹ️ Không cần | Con số `unwrap` chủ yếu test/bất khả lỗi — không sửa. |

**Thay đổi hạ tầng kèm theo:** CI integration job khởi động NATS bằng `docker run ... nats:2.10 -js`
(service container không cho override command), giữ nguyên các gate khác. `deploy/docker-compose.dev.yml`
đã bật sẵn `-js` từ trước.

---

## 12. Khắc phục blocker Go-Live (đã triển khai trong PR này)

Các blocker hạ tầng nêu ở đánh giá sẵn sàng go-live đã được vá ở tầng manifest k8s + observability:

| Blocker | Trạng thái | Thay đổi |
|---|---|---|
| NATS JetStream không bền (mất dữ liệu khi restart) | ✅ | `deploy/k8s/30-nats.yaml`: **StatefulSet 3 node, JetStream cluster, PVC `/data` mỗi node**, headless service, PDB `minAvailable: 2` (giữ quorum RAFT). Stream tạo với **R3** qua `JETSTREAM_REPLICAS=3` (`10-config.yaml`); code đọc env, **mặc định 1** nên dev/CI single-node vẫn chạy. |
| Postgres không backup | ✅ | `20-postgres.yaml`: **CronJob `postgres-backup`** (pg_dump → gzip → PVC riêng, prune >7 ngày, RPO ~1 ngày). HA thật (failover) khuyến nghị dùng operator CloudNativePG/managed — đã ghi chú. |
| Mọi service `replicas: 1` (không HA) | ✅ | `40-services.yaml` + verticals: **`replicas: 2`**; thêm **PodDisruptionBudget** (`45-pdb.yaml`) và **HPA 2→5 theo CPU** (`46-hpa.yaml`). Scale relay an toàn nhờ dedup `Nats-Msg-Id`. |
| Network policy cho hạ tầng mới | ✅ | `60-network-policy.yaml`: mở route NATS↔NATS `6222` (ingress + egress) để cluster hình thành; cho backup job egress tới Postgres `5432`. |
| Cảnh báo DLQ | ✅ | `deploy/observability/alerts.yaml`: `DigicoreEventDeadLettered` (critical) khi có event bị đẩy DLQ. |

## 13. Nhóm should-have (đã triển khai tiếp)

| Hạng mục | Trạng thái | Thay đổi |
|---|---|---|
| Lockout brute-force login | ✅ | auth-svc: cổng `LoginAttemptRepository` + `auth_svc.login_attempts` (migration 0004) + logic khóa **5 lần sai → khóa 15 phút** (`services.rs`), repo Postgres atomic (`login_attempt_repo_pg.rs`). Bật trong wiring; **4 unit test** (khóa, chặn khi đã khóa dù đúng mật khẩu, reset khi thành công). Khóa chia sẻ across replica (lưu DB). |
| HA Postgres thật | ✅ (opt-in) | Overlay **CloudNativePG** `deploy/k8s/ha-postgres/`: Cluster 3 instance + failover + WAL archiving/PITR + ScheduledBackup; giữ `DATABASE_URL` qua Service ExternalName `postgres` → `digicore-pg-rw`. Tách khỏi kustomization gốc (chọn dùng). |
| CD tự động | ✅ (template) | `.github/workflows/deploy.yml`: `workflow_dispatch` (thủ công, không chạy khi push → không ảnh hưởng CI), build+push 8 image, `kustomize apply`, gate qua environment `production`. Cần điền secrets registry/kubeconfig. |

## 14. Hoàn tất nhóm should-have còn lại

| Hạng mục | Trạng thái | Thay đổi |
|---|---|---|
| Secret management nâng cao | ✅ (opt-in) | `deploy/k8s/secrets/`: ví dụ **SealedSecret** (Bitnami) và **ExternalSecret** (External Secrets Operator → Vault/AWS/GCP) cho `digicore-postgres` + `digicore-jwt`, kèm README hướng dẫn bỏ secret dev khỏi `10-config.yaml` và xoay vòng khóa. |
| Load/perf test | ✅ | `deploy/load/k6-smoke.js`: k6 chạy login + read thật, threshold `p95<500ms`, `http_req_failed<1%` (fail → exit≠0, gate được). README hướng dẫn dùng kết quả để tinh chỉnh resource/HPA. |
| Tinh chỉnh resource | ✅ (quy trình) | Không đổi mù số liệu; README load-test mô tả cách đọc kết quả k6 để chỉnh `requests/limits` + `maxReplicas` (đã có HPA 2→5). |
| Rollback migration | ✅ (chiến lược) | `docs/MIGRATIONS.md`: sqlx forward-only → **expand/contract** + khôi phục từ backup (logical dump hoặc PITR của CNPG) + backup ngay trước deploy có migration. |

> Lưu ý kiểm chứng: code (lockout) pass clippy/test; k6 script pass kiểm tra cú pháp; mọi YAML mới validate cú pháp nhưng **chưa `kubectl apply`/`kustomize build` trên cluster thật** — cần kiểm thử staging trước go-live. Migration `0004` + repo Postgres chạy ở job `integration` của CI.

> **Kết luận go-live:** tất cả blocker (§12) và should-have (§13–14) đã được xử lý ở mức code + manifest/template + tài liệu. Việc còn lại thuần vận hành: cung cấp secret thật, cài operator (CNPG/Sealed/External Secrets), chạy k6 trên staging để chốt resource, và `kustomize build`/`apply` kiểm thử trên cluster trước khi GA.
