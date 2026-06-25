import { expect, test } from "@playwright/test";

import { TEST_IDS } from "../../src/testing/testIds";

test("synthetic demo renders and opens a public-safe image preview", async ({ page }) => {
  await page.goto("/?demo=backup-chat");

  const image = page.getByRole("img", { name: "demo-photo.jpg" });
  await expect(image).toBeVisible();

  const naturalSize = await image.evaluate((element) => ({
    height: element.naturalHeight,
    width: element.naturalWidth,
  }));
  const thumbnailBox = await image.boundingBox();

  expect(naturalSize.width).toBeGreaterThan(0);
  expect(naturalSize.height).toBeGreaterThan(0);
  expect(thumbnailBox).not.toBeNull();

  await image.click();

  const dialog = page.getByRole("dialog", { name: "demo-photo.jpg" });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByText("demo-photo.jpg")).toBeVisible();

  const previewBox = await dialog.getByRole("img", { name: "demo-photo.jpg" }).boundingBox();
  expect(previewBox).not.toBeNull();
  expect(previewBox?.width).toBeGreaterThan((thumbnailBox?.width ?? 0) * 1.5);
  await expect(page.getByTestId(TEST_IDS.mediaBlock)).toHaveCount(2);
});
