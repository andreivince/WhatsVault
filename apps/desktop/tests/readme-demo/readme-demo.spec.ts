import { expect, test } from "@playwright/test";

import { hasPrivateDemoText } from "../../scripts/privacy-rules.mjs";

import { TEST_IDS } from "../../src/testing/testIds";

const DEMO_STEP_PAUSE_MS = 4_200;

test("README demo walkthrough", async ({ page }) => {
  await test.step("Open the synthetic WhatsVault demo", async () => {
    await page.goto("/?demo=1");

    await expect(page.getByTestId(TEST_IDS.appShell)).toBeVisible();
    const bodyText = await page.evaluate(() => document.body.textContent ?? "");
    expect(hasPrivateDemoText(bodyText)).toBe(false);
    await expect(page.getByTestId(TEST_IDS.chatTitle)).toHaveText("Design Preview");
    await expect(page.getByTestId(TEST_IDS.conversationHeader)).toContainText("9 messages");
    await page.waitForTimeout(DEMO_STEP_PAUSE_MS);
  });

  await test.step("Search local messages", async () => {
    await page.getByTestId(TEST_IDS.searchInput).fill("local");

    await expect(page.getByText("The files stay local on this computer.")).toBeVisible();
    await expect(page.getByText("No messages match this search.")).toBeHidden();
    await page.waitForTimeout(DEMO_STEP_PAUSE_MS);
  });

  await test.step("Show media in the conversation", async () => {
    await page.getByTestId(TEST_IDS.searchInput).fill("");

    await expect(page.getByTestId(TEST_IDS.mediaBlock).first()).toBeVisible();
    await expect(page.getByRole("img", { name: "demo-photo.jpg" })).toBeVisible();
    await expect(page.getByText("Voice message", { exact: true })).toBeVisible();
    await page.waitForTimeout(DEMO_STEP_PAUSE_MS);
  });

  await test.step("Focus export controls", async () => {
    await page.getByTestId(TEST_IDS.exportButton).hover();

    await expect(page.getByLabel("Export chat to HTML")).toBeEnabled();
    await page.waitForTimeout(DEMO_STEP_PAUSE_MS);
  });
});
