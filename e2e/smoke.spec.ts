import { test, expect, type Page } from "@playwright/test";

// A demo session is read straight from localStorage by the AuthProvider, so we
// can land on protected routes without a real auth-svc.
const SESSION = {
  token: "demo.e2e.token",
  user: { id: "u-demo", tenant_id: "t-demo", roles: ["OWNER"] },
};

const now = new Date().toISOString();

const loyaltyList = {
  page: 1,
  page_size: 50,
  total: 2,
  items: [
    { tenant_id: "t-demo", customer_id: "KH-1001", points_balance: 12450, lifetime_spend: 1245000, tier: "GOLD", updated_at: now },
    { tenant_id: "t-demo", customer_id: "KH-1002", points_balance: 540, lifetime_spend: 54000, tier: "BRONZE", updated_at: now },
  ],
};
const loyaltyRules = { minor_per_point: 100, silver_min: 100000, gold_min: 1000000 };
const shipmentsList = {
  page: 1,
  page_size: 50,
  total: 1,
  items: [
    { id: "00000000-0000-7000-8000-000000000001", tenant_id: "t-demo", reference: "EXP-1A2B3C4D", destination_country: "US", incoterm: "FOB", status: "BOOKED", order_id: "DH-1", created_at: now },
  ],
};

// Seed the session and stub every API call with canned JSON.
async function seed(page: Page) {
  await page.addInitScript((s) => {
    localStorage.setItem("digicore.auth", JSON.stringify(s));
  }, SESSION);
  await page.route("**/api/v1/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    let body: unknown = [];
    if (path.endsWith("/retail/loyalty")) body = loyaltyList;
    else if (path.endsWith("/loyalty/rules")) body = loyaltyRules;
    else if (path.endsWith("/trade-export/shipments")) body = shipmentsList;
    return route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(body),
    });
  });
}

test("login page renders", async ({ page }) => {
  await page.goto("/login");
  await expect(page.getByRole("heading", { name: "DigicoreOS" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Đăng nhập" })).toBeVisible();
});

test("loyalty screen lists accounts and shows the rules panel", async ({ page }) => {
  await seed(page);
  await page.goto("/loyalty");
  await expect(page.getByText("KH-1001")).toBeVisible();
  await expect(page.getByText("KH-1002")).toBeVisible();
  await expect(page.getByText(/Quy tắc:/)).toBeVisible();
});

test("shipments screen lists shipments", async ({ page }) => {
  await seed(page);
  await page.goto("/shipments");
  await expect(page.getByText("EXP-1A2B3C4D")).toBeVisible();
  await expect(page.getByText("BOOKED")).toBeVisible();
});
