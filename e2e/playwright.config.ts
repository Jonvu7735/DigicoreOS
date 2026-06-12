import { defineConfig } from "@playwright/test";

// Point at a specific Chromium when Playwright's managed download isn't
// available (e.g. CI sandboxes); otherwise use the one `playwright install`
// fetched.
const executablePath = process.env.PW_EXECUTABLE_PATH || undefined;

export default defineConfig({
  testDir: ".",
  timeout: 30_000,
  fullyParallel: true,
  use: {
    baseURL: "http://localhost:4173",
    launchOptions: {
      executablePath,
      args: ["--no-sandbox", "--disable-gpu", "--disable-dev-shm-usage"],
    },
  },
  // Build the SPA and serve it with Vite's preview server. The tests mock the
  // API, so no backend services are needed.
  webServer: {
    command:
      "npm --prefix ../frontend run build && npm --prefix ../frontend run preview -- --port 4173 --strictPort",
    url: "http://localhost:4173",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
