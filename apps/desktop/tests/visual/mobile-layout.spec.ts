import { expect, test, type Page } from "@playwright/test";

import { TEST_IDS } from "../../src/testing/testIds";

async function expectNoHorizontalOverflow(page: Page) {
  const metrics = await page.evaluate(() => ({
    bodyWidth: document.body.scrollWidth,
    viewportWidth: window.innerWidth,
    documentWidth: document.documentElement.scrollWidth,
  }));

  expect(metrics.bodyWidth).toBeLessThanOrEqual(metrics.viewportWidth);
  expect(metrics.documentWidth).toBeLessThanOrEqual(metrics.viewportWidth);
}

test.use({
  viewport: { width: 390, height: 844 },
  deviceScaleFactor: 1,
});

test("mobile backup chat keeps the selected avatar clear of the viewport edge", async ({ page }) => {
  await page.goto("/?demo=backups");

  await page.locator(".backup-row.openable").first().click();
  await page.locator(".backup-chat-row").first().click();

  await expect(page.getByTestId(TEST_IDS.conversationHeader)).toBeVisible();
  await expect(page.getByTestId(TEST_IDS.chatTitle)).toHaveText("Design Preview");

  const avatarBox = await page.locator(".conversation-header .avatar").boundingBox();
  expect(avatarBox).not.toBeNull();
  expect(avatarBox?.x).toBeGreaterThanOrEqual(18);
  expect(avatarBox?.y).toBeGreaterThanOrEqual(0);
  expect(avatarBox?.y).toBeLessThan(120);
  await expectNoHorizontalOverflow(page);
});

test("mobile backup picker stays within the viewport", async ({ page }) => {
  await page.goto("/?demo=backups");

  await expect(page.getByText("iPhone backups")).toBeVisible();
  await expect(page.getByText("Demo iPhone")).toBeVisible();
  await expectNoHorizontalOverflow(page);
});

test("mobile backup picker filters chat rows from the shared search box", async ({ page }) => {
  await page.goto("/?demo=backups");

  const sidebar = page.locator(".chat-sidebar");
  await page.getByTestId(TEST_IDS.searchInput).fill("project ready");

  await expect(sidebar.getByText("Project Archive")).toBeVisible();
  await expect(sidebar.getByText("Design Preview")).toBeHidden();
  await expect(sidebar.getByText("Media Archive")).toBeHidden();
  await expectNoHorizontalOverflow(page);

  await page.getByTestId(TEST_IDS.searchInput).fill("no matching archive");
  await expect(sidebar.getByText("No chats match this search.")).toBeVisible();
  await expectNoHorizontalOverflow(page);
});

test("mobile backup chat filters visible messages by date", async ({ page }) => {
  await page.goto("/?demo=backup-chat");

  await page.getByTestId(TEST_IDS.dateFilterInput).fill("2026-06-23");
  await expect(page.getByText("Viewing local backup")).toBeVisible();
  await expect(page.getByText("It is wild how much context is locked inside a backup.")).toBeVisible();
  await expect(page.getByText("No messages match these filters.")).toBeHidden();
  await expectNoHorizontalOverflow(page);

  await page.getByTestId(TEST_IDS.dateFilterInput).fill("2026-06-24");
  await expect(page.getByText("No messages match these filters.")).toBeVisible();
  await expectNoHorizontalOverflow(page);
});
