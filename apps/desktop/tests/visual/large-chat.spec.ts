import { expect, test } from "@playwright/test";

import { TEST_IDS } from "../../src/testing/testIds";

test("large synthetic chats render a bounded recent window before expanding older messages", async ({ page }) => {
  await page.goto("/?demo=large-chat");

  await expect(page.getByTestId(TEST_IDS.chatTitle)).toHaveText("Large Archive");
  await expect(page.getByTestId(TEST_IDS.messageBubble)).toHaveCount(420);
  await expect(page.getByTestId(TEST_IDS.showEarlierButton)).toHaveText("Show 420 earlier messages");

  await page.getByTestId(TEST_IDS.showEarlierButton).click();

  await expect(page.getByTestId(TEST_IDS.messageBubble)).toHaveCount(840);
  await expect(page.getByTestId(TEST_IDS.showEarlierButton)).toHaveText("Show 60 earlier messages");
});
