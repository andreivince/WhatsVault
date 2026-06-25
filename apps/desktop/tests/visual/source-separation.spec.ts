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
  await expect(backupSource).toContainText("Preview ready");
  await expect(backupSource).toContainText("Real backup chats render locally");
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

test("unavailable backup rows expand with plain next steps", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/?demo=backups");

  await page.getByRole("button", { name: /Travel Phone/ }).click();
  const unavailableDrawer = page.locator(".backup-chat-drawer.unavailable");
  await expect(unavailableDrawer.getByText("WhatsApp not found")).toBeVisible();
  await expect(
    unavailableDrawer.getByText("WhatsApp data was not found in this backup."),
  ).toBeVisible();
  await expect(unavailableDrawer.getByRole("button", { name: "Refresh" })).toBeVisible();

  await page.getByRole("button", { name: /Encrypted iPhone/ }).click();
  await expect(unavailableDrawer.getByText("Encrypted backup", { exact: true })).toBeVisible();
  await expect(
    unavailableDrawer.getByText("Choose an unencrypted backup or make a new unencrypted local backup."),
  ).toBeVisible();
  const privacyNoteDoesNotOverlapPanel = await page.evaluate(() => {
    const panel = document.querySelector(".backup-panel");
    const note = document.querySelector(".encryption-note");
    if (!panel || !note) {
      throw new Error("Missing backup panel or privacy note");
    }

    const panelBox = panel.getBoundingClientRect();
    const noteBox = note.getBoundingClientRect();
    return noteBox.bottom <= panelBox.top || noteBox.top >= panelBox.bottom;
  });
  expect(privacyNoteDoesNotOverlapPanel).toBe(true);

  const layout = await page.evaluate(() => ({
    bodyScrollWidth: document.body.scrollWidth,
    viewportWidth: window.innerWidth,
  }));
  expect(layout.bodyScrollWidth).toBeLessThanOrEqual(layout.viewportWidth);
});
