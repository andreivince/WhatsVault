import { expect, test } from "@playwright/test";

import { TEST_IDS } from "../../src/testing/testIds";

test("source screen separates supported ZIP viewing from iPhone backup proof work", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByTestId(TEST_IDS.appShell)).toBeVisible();
  await expect(page.getByTestId(TEST_IDS.sourceOverview)).toBeVisible();

  const zipSource = page.getByTestId(TEST_IDS.supportedSourceCard);
  await expect(zipSource).toContainText("WhatsApp export ZIP");
  await expect(zipSource).toContainText("Available now");
  await expect(zipSource.getByRole("button", { name: "Open WhatsApp export ZIP" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Open WhatsApp export ZIP" })).toHaveCount(1);

  const backupSource = page.getByTestId(TEST_IDS.proofSourceCard);
  await expect(backupSource).toContainText("iPhone backup");
  await expect(backupSource).toContainText("Proof work");
  await expect(backupSource).toContainText("Real-backup proof pending");
  await expect(backupSource.getByRole("button")).toHaveCount(0);
  await expect(page.locator(".source-card-icon")).toHaveCount(0);
  await expect(page.locator(".empty-mark")).toHaveCount(0);
  await expect(page.locator(".sidebar-empty svg")).toHaveCount(0);
  await expect(page.locator(".backup-panel-header > span svg")).toHaveCount(0);
  await expect(page.locator(".encryption-note svg")).toHaveCount(0);

  const layout = await page.evaluate(() => ({
    bodyScrollWidth: document.body.scrollWidth,
    viewportWidth: window.innerWidth,
  }));
  expect(layout.bodyScrollWidth).toBeLessThanOrEqual(layout.viewportWidth);

  await expect(page.getByLabel("Detected iPhone backups")).toBeVisible();
  await expect(page.getByText("No local iPhone backups detected")).toBeVisible();
});

test("source screen fits the default in-app browser width", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");

  await expect(page.getByTestId(TEST_IDS.sourceOverview)).toBeVisible();
  await expect(page.getByRole("button", { name: "Open WhatsApp export ZIP" })).toHaveCount(1);

  const layout = await page.evaluate(() => ({
    bodyScrollWidth: document.body.scrollWidth,
    viewportWidth: window.innerWidth,
  }));
  expect(layout.bodyScrollWidth).toBeLessThanOrEqual(layout.viewportWidth);
});
