import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  use: { baseURL: "http://localhost:5173", trace: "off", viewport: { width: 1280, height: 720 } },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"], viewport: { width: 1280, height: 720 } } }],
  webServer: { command: "npm run dev:mock", port: 5173, reuseExistingServer: true, timeout: 60_000 },
});
