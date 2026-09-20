import { expect, test } from "@playwright/test";

import { TEST_IDS } from "../../src/testing/testIds";

test("large synthetic chats keep DOM rendering bounded while expanding older messages", async ({ page }) => {
  await page.goto("/?demo=large-chat");

  await expect(page.getByTestId(TEST_IDS.chatTitle)).toHaveText("Large Archive");
  await expect(page.getByText("900 recent messages loaded")).toBeVisible();
  await expect(page.getByText("Search can find older messages in this backup. Export uses the loaded recent messages.")).toBeVisible();
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

test("mixed-height messages remain anchored during small scroll movements", async ({ page }) => {
  await page.addInitScript(() => {
    const messages = Array.from({ length: 420 }, (_, index) => ({
      id: `mixed-${index}`, sender: "Example", timestamp: { raw: "2026-06-23T12:00:00Z" },
      body: `Mixed message ${index}.\n${"A longer message line.\n".repeat(index % 3 === 0 ? 22 : 2)}`,
      attachment_ids: [],
    }));
    Object.assign(window, { __TAURI_INTERNALS__: { invoke(command: string) {
      if (command === "list_iphone_backups") return Promise.resolve([]);
      if (command === "open_whatsapp_export") return Promise.resolve({
        source: { kind: "whatsapp_export_zip", handle: "mixed-source", displayName: "Mixed archive" },
        imported: { source_kind: "whatsapp_export_zip", transcript_name: "Mixed archive", messages, attachments: [], issues: [] },
      });
      return Promise.resolve(null);
    } } });
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Open WhatsApp export ZIP", exact: true }).click();
  await expect(page.getByTestId(TEST_IDS.chatTitle)).toHaveText("Mixed archive");
  await expect(page.getByTestId(TEST_IDS.virtualMessageList).getByText(/^Mixed message 419\./)).toBeInViewport();
  const canvas = page.getByTestId(TEST_IDS.messageCanvas);
  await canvas.evaluate(element => { element.scrollTop = 8000; });
  // Wait for scrolling and ResizeObserver measurement to settle before capturing an anchor.
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  const anchor = await canvas.evaluate(element => {
    const viewport = element.getBoundingClientRect();
    const row = Array.from(element.querySelectorAll<HTMLElement>(".message-row")).find(item => {
      const box = item.getBoundingClientRect();
      return box.top < viewport.top + viewport.height / 2 && box.bottom > viewport.top + viewport.height / 2;
    });
    return row ? { text: row.querySelector("p")!.textContent!, top: row.getBoundingClientRect().top } : null;
  });
  expect(anchor).not.toBeNull();
  const anchorRow = page.getByTestId(TEST_IDS.messageBubble).filter({ hasText: anchor!.text.split("\n")[0] });
  for (let step = 1; step <= 4; step++) {
    await canvas.evaluate(element => { element.scrollTop += 100; });
    await expect.poll(async () => (await anchorRow.boundingBox())?.y).toBeCloseTo(anchor!.top - step * 100, 0);
  }
  await expect.poll(() => page.getByTestId(TEST_IDS.messageBubble).count()).toBeLessThanOrEqual(120);
});
