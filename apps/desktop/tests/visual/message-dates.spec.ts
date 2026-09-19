import { expect, test } from "@playwright/test";

test("archive date headings describe the messages and disappear for empty results", async ({ page }) => {
  await page.goto("/?demo=backup-chat");
  await expect(page.locator(".day-pill")).toHaveText("Jun 23, 2026");
  await page.getByLabel("Filter messages by date").fill("2026-06-24");
  await expect(page.getByText("No messages match these filters.")).toBeVisible();
  await expect(page.locator(".day-pill")).toHaveCount(0);
});
