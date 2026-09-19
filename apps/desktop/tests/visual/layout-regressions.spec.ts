import { expect, test } from "@playwright/test";

for (const width of [1040, 1440]) {
  test(`source picker remains reachable in a ${width} by 680 window`, async ({ page }) => {
    await page.setViewportSize({ width, height: 680 });
    await page.goto("/?demo=backups");
    await expect(page.getByRole("heading", { name: "WhatsVault", exact: true })).toBeInViewport();
    const lastBackup = page.getByRole("button", { name: /Encrypted iPhone/ });
    await lastBackup.scrollIntoViewIfNeeded();
    await expect(lastBackup).toBeInViewport({ ratio: 1 });
    await lastBackup.click();
    const guidance = page.getByText("WhatsVault cannot open this backup yet.", { exact: false });
    await guidance.scrollIntoViewIfNeeded();
    await expect(guidance).toBeInViewport({ ratio: 1 });
  });
}

test("brand asset is visible and window controls do not cover chat actions", async ({ page }) => {
  await page.addInitScript(() => Object.defineProperty(window, "__TAURI_INTERNALS__", {
    value: { invoke: async () => null },
  }));
  await page.goto("/?demo=backup-chat");
  const controls = await page.getByRole("navigation", { name: "Window controls" }).boundingBox();
  const header = await page.locator(".conversation-header").boundingBox();
  expect(controls!.y + controls!.height).toBeLessThanOrEqual(header!.y);
  const brand = page.getByRole("img", { name: "WhatsVault icon" });
  await expect(brand).toBeVisible();
  expect(await brand.evaluate((image: HTMLImageElement) => image.naturalWidth)).toBeGreaterThan(0);
});

test("narrow search status and date controls stay inside the conversation", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/?demo=large-chat");
  await page.getByLabel("Filter messages by date").fill("2026-06-24");
  await expect(page.locator(".banner-state")).toHaveText("0 matches in loaded messages");
  const overflow = await page.locator(".banner-tools").evaluate((tools) => {
    const parent = tools.getBoundingClientRect();
    return Array.from(tools.children).some((child) => child.getBoundingClientRect().right > parent.right + 1);
  });
  expect(overflow).toBe(false);
});

test("portrait media preserves its aspect ratio inside the bubble", async ({ page }) => {
  await page.goto("/?demo=backup-chat");
  const image = page.getByRole("img", { name: "demo-photo.jpg" });
  const dimensions = await image.evaluate(async (image: HTMLImageElement) => {
    image.src = 'data:image/svg+xml,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="200" height="1000"><rect width="200" height="1000" fill="green"/></svg>');
    await image.decode();
    const box = image.getBoundingClientRect();
    return { rendered: box.width / box.height, natural: image.naturalWidth / image.naturalHeight };
  });
  expect(dimensions.rendered).toBeCloseTo(dimensions.natural, 2);
});

test("image preview keeps keyboard focus inside and restores it on close", async ({ page }) => {
  await page.goto("/?demo=backup-chat");
  const opener = page.getByRole("button", { name: "Open demo-photo.jpg" });
  await opener.click();
  const dialog = page.getByRole("dialog", { name: "demo-photo.jpg" });
  await expect(dialog).toBeVisible();
  await expect(dialog.locator(".preview-close")).toBeFocused();
  await page.keyboard.press("Tab");
  // Native dialogs may give browser chrome a Tab stop, but the app behind is inert.
  const search = page.getByLabel("Search chats and messages");
  await search.evaluate((input: HTMLInputElement) => input.focus());
  await expect(search).not.toBeFocused();
  await page.keyboard.press("Tab");
  await expect(search).not.toBeFocused();
  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await expect(opener).toBeFocused();
});
