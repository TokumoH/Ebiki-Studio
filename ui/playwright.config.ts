import { defineConfig, devices } from "@playwright/test";

// E2E_PORT permite rodar em paralelo com outro servidor de desenvolvimento (padrão 5173).
const port = Number(process.env.E2E_PORT ?? 5173);

export default defineConfig({
  testDir: "./e2e",
  timeout: 30_000,
  use: { baseURL: `http://localhost:${port}`, trace: "off", viewport: { width: 1280, height: 720 } },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"], viewport: { width: 1280, height: 720 } } }],
  webServer: { command: `npm run dev:mock -- --port ${port}`, port, reuseExistingServer: true, timeout: 60_000 },
});
