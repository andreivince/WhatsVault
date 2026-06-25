import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/readme-demo",
  outputDir: "./target/readme-demo/test-results",
  reporter: [["line"]],
  timeout: 45_000,
  use: {
    ...devices["Desktop Chrome"],
    baseURL: "http://127.0.0.1:1420",
    colorScheme: "light",
    screenshot: "only-on-failure",
    trace: "on",
    video: {
      mode: "on",
      size: { width: 1440, height: 920 },
    },
    viewport: { width: 1440, height: 920 },
  },
  webServer: {
    command: "npm run dev -- --host 127.0.0.1 --port 1420",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    url: "http://127.0.0.1:1420/?demo=1",
  },
});
