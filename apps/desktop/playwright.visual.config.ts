import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/visual",
  outputDir: "./target/visual-test-results",
  reporter: [["line"]],
  timeout: 30_000,
  use: {
    ...devices["Desktop Chrome"],
    baseURL: "http://127.0.0.1:1420",
    colorScheme: "light",
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
    viewport: { width: 1440, height: 920 },
  },
  webServer: {
    command: "npm run dev -- --host 127.0.0.1 --port 1420",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    url: "http://127.0.0.1:1420/?demo=backup-chat",
  },
});
