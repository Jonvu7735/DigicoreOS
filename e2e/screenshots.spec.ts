// Captures full-page screenshots of every screen for visual review. Reuses the
// smoke-test approach: seed a demo session into localStorage and stub every
// /api/v1/* call with canned JSON, so no backend is needed.
//
// Run: PW_EXECUTABLE_PATH=/opt/pw-browsers/chromium-1194/chrome-linux/chrome \
//      npx playwright test screenshots.spec.ts
import { test, type Page } from "@playwright/test";

const SESSION = {
  token: "demo.e2e.token",
  user: { id: "u-demo", tenant_id: "t-demo", roles: ["OWNER"] },
};
const now = new Date().toISOString();

const loyaltyList = {
  page: 1, page_size: 50, total: 2,
  items: [
    { tenant_id: "t-demo", customer_id: "KH-1001", points_balance: 12450, lifetime_spend: 1245000, tier: "GOLD", updated_at: now },
    { tenant_id: "t-demo", customer_id: "KH-1002", points_balance: 540, lifetime_spend: 54000, tier: "BRONZE", updated_at: now },
  ],
};
const loyaltyRules = { minor_per_point: 100, silver_min: 100000, gold_min: 1000000 };
const account = { tenant_id: "t-demo", customer_id: "KH-1001", points_balance: 12450, lifetime_spend: 1245000, tier: "GOLD", updated_at: now };
const ledger = [
  { id: "l1", kind: "EARN", points: 2450, balance_after: 12450, reason: "order:DH-2087", at: now },
  { id: "l2", kind: "REDEEM", points: 1000, balance_after: 10000, reason: "voucher:TET2026", at: now },
  { id: "l3", kind: "EARN", points: 10000, balance_after: 11000, reason: "order:DH-1990", at: now },
];
const shipmentsList = {
  page: 1, page_size: 50, total: 2,
  items: [
    { id: "00000000-0000-7000-8000-000000000001", tenant_id: "t-demo", reference: "EXP-1A2B3C4D", destination_country: "US", incoterm: "FOB", status: "BOOKED", order_id: "DH-1", created_at: now },
    { id: "00000000-0000-7000-8000-000000000002", tenant_id: "t-demo", reference: "EXP-9Z8Y7X6W", destination_country: "DE", incoterm: "CIF", status: "DISPATCHED", order_id: "DH-2", created_at: now },
  ],
};
const shipment = { id: "00000000-0000-7000-8000-000000000001", tenant_id: "t-demo", reference: "EXP-1A2B3C4D", destination_country: "US", incoterm: "FOB", status: "BOOKED", order_id: "DH-1", created_at: now };
const cargo = [
  { id: "c1", description: "Cà phê Robusta rang", hs_code: "0901.21", quantity: 200, unit: "CTN", net_weight_kg: 3200 },
  { id: "c2", description: "Hạt điều W320", hs_code: "0801.32", quantity: 80, unit: "BAG", net_weight_kg: 2000 },
];
const history = [
  { id: "h1", from_status: null, to_status: "DRAFT", at: now },
  { id: "h2", from_status: "DRAFT", to_status: "BOOKED", at: now },
];

async function seed(page: Page) {
  await page.addInitScript((s) => localStorage.setItem("digicore.auth", JSON.stringify(s)), SESSION);
  await page.route("**/api/v1/**", (route) => {
    const p = new URL(route.request().url()).pathname;
    const post = route.request().method() === "POST";
    let body: unknown = [];
    if (p.endsWith("/reporting/overview"))
      body = { report: "overview", generated_at: now, data: { revenue: 1245000000, orders: 1284, customers: 342, products: 87, inventory_units: 15600, employees: 24 } };
    else if (post && p.endsWith("/ai/query"))
      body = { answer: "Để đặt chỗ một lô hàng: vào màn Shipments → Tạo lô hàng (nước đến + Incoterm), rồi nhấn 'Đặt chỗ' để chuyển sang trạng thái BOOKED. Sau đó thêm dòng hàng (packing list) ở màn chi tiết.", model: "stub-assistant" };
    else if (post && p.endsWith("/crm/customers")) body = { id: "c-demo", name: "Demo Khách 482931" };
    else if (post && p.endsWith("/erp/products")) body = { id: "p-demo" };
    else if (post && p.endsWith("/erp/orders")) body = { id: "o-demo", total_amount: 250000 };
    else if (p.endsWith("/retail/loyalty")) body = loyaltyList;
    else if (/\/retail\/loyalty\/[^/]+\/ledger$/.test(p)) body = ledger;
    else if (p.endsWith("/loyalty/rules")) body = loyaltyRules;
    else if (/\/retail\/loyalty\/[^/]+$/.test(p)) body = account;
    else if (p.endsWith("/trade-export/shipments")) body = shipmentsList;
    else if (/\/shipments\/[^/]+\/cargo$/.test(p)) body = cargo;
    else if (/\/shipments\/[^/]+\/history$/.test(p)) body = history;
    else if (/\/shipments\/[^/]+$/.test(p)) body = shipment;
    return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(body) });
  });
}

test.use({ viewport: { width: 1280, height: 900 } });

test("capture all screens", async ({ page }) => {
  const shot = (name: string) => page.screenshot({ path: `shots/${name}.png`, fullPage: true });

  // Login (no session needed).
  await page.goto("/login");
  await page.waitForSelector("text=Đăng nhập để tiếp tục");
  await shot("01-login");

  await seed(page);

  await page.goto("/");
  await page.waitForSelector("text=Lối tắt");
  await shot("02-home");

  await page.goto("/loyalty");
  await page.waitForSelector("text=KH-1001");
  await shot("03-loyalty");

  await page.goto("/loyalty/KH-1001");
  await page.waitForSelector("text=Lịch sử điểm");
  await shot("04-loyalty-detail");

  await page.goto("/shipments");
  await page.waitForSelector("text=EXP-1A2B3C4D");
  await shot("05-shipments");

  await page.goto("/shipments/00000000-0000-7000-8000-000000000001");
  await page.waitForSelector("text=Dòng hàng");
  await shot("06-shipment-detail");

  await page.goto("/assistant");
  await page.fill("input", "cách đặt chỗ lô hàng?");
  await page.getByRole("button", { name: "Hỏi" }).click();
  await page.waitForSelector(".done-box");
  await shot("07-assistant");

  await page.goto("/demo");
  await page.getByRole("button", { name: "Tạo đơn hàng demo" }).click();
  await page.waitForSelector("text=Xong!");
  await shot("08-demo");
});
