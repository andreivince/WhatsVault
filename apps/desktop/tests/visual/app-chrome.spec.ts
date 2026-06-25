import { expect, test } from "@playwright/test";

import { TEST_IDS } from "../../src/testing/testIds";

test("desktop shell does not render platform-specific fake window controls", async ({ page }) => {
  await page.goto("/?demo=backup-chat");

  await expect(page.getByTestId(TEST_IDS.appShell)).toBeVisible();
  await expect(page.locator(".window-dots")).toHaveCount(0);
  await expect(page.locator(".navigation-rail")).toHaveCount(0);
  await expect(page.locator(".composer")).toHaveCount(0);
  await expect(page.locator(".sidebar-header .icon-button")).toHaveCount(0);
  await expect(page.getByLabel("Video call")).toHaveCount(0);
  await expect(page.getByLabel("Call")).toHaveCount(0);
  await expect(page.getByLabel("Add")).toHaveCount(0);
  await expect(page.getByLabel("Emoji")).toHaveCount(0);
  await expect(page.getByLabel("Voice")).toHaveCount(0);
});

test("chat chrome does not expose fake or unsupported actions", async ({ page }) => {
  await page.goto("/?demo=backup-chat");

  await expect(page.getByTestId(TEST_IDS.appShell)).toBeVisible();
  await expect(page.locator(".lucide-pin")).toHaveCount(0);
  await expect(page.locator(".lucide-check-check")).toHaveCount(0);

  await page.goto("/?demo=1");

  await expect(page.getByTestId(TEST_IDS.appShell)).toBeVisible();
  await expect(page.locator("button.chat-row.selected")).toHaveCount(0);
});

test("passive fields do not render decorative action icons", async ({ page }) => {
  await page.goto("/?demo=backup-chat");

  await expect(page.getByTestId(TEST_IDS.appShell)).toBeVisible();
  await expect(page.locator(".search-box svg")).toHaveCount(0);
  await expect(page.locator(".date-filter svg")).toHaveCount(0);
  await expect(page.locator(".attachment-chip svg")).toHaveCount(0);
});
