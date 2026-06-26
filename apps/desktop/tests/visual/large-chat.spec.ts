import { expect, test } from "@playwright/test";

import { TEST_IDS } from "../../src/testing/testIds";

test("large synthetic chats keep DOM rendering bounded while expanding older messages", async ({ page }) => {
  await page.goto("/?demo=large-chat");

  await expect(page.getByTestId(TEST_IDS.chatTitle)).toHaveText("Large Archive");
  await expect(page.getByText("900 recent messages loaded")).toBeVisible();
  await expect(page.getByText("Search and export use the loaded recent messages.")).toBeVisible();
  await expect(page.getByTestId(TEST_IDS.virtualMessageList)).toHaveAttribute("data-total-messages", "420");
  await expect.poll(async () => page.getByTestId(TEST_IDS.messageBubble).count()).toBeLessThanOrEqual(120);
  await expect(page.getByText("Large archive synthetic message 900.")).toBeVisible();
  await expect(page.getByTestId(TEST_IDS.showEarlierButton)).toHaveText("Show 420 earlier messages");

  await page.getByTestId(TEST_IDS.showEarlierButton).click();

  await expect(page.getByTestId(TEST_IDS.virtualMessageList)).toHaveAttribute("data-total-messages", "840");
  await expect.poll(async () => page.getByTestId(TEST_IDS.messageBubble).count()).toBeLessThanOrEqual(120);
  await expect(page.getByTestId(TEST_IDS.showEarlierButton)).toHaveText("Show 60 earlier messages");

  await page.getByTestId(TEST_IDS.messageCanvas).evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });
  await expect(page.getByText("Large archive synthetic message 900.")).toBeVisible();
});
