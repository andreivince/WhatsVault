import { mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

import { hasPrivateDemoText } from "./privacy-rules.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appDir = dirname(scriptDir);
const repoRoot = resolve(appDir, "../..");
const screenshotPath = join(repoRoot, "docs", "assets", "whatsvault-synthetic-demo.png");
const demoUrl = process.env.WHATSVAULT_DEMO_URL ?? "http://127.0.0.1:1420/?demo=backup-chat";
const screenshotViewport = { width: 1440, height: 920 };

mkdirSync(dirname(screenshotPath), { recursive: true });

const browser = await chromium.launch();
const page = await browser.newPage({
  viewport: screenshotViewport,
  deviceScaleFactor: 1,
});

try {
  await page.goto(demoUrl);
  await page.waitForSelector('[data-testid="app-shell"]');
  await page.waitForSelector('[data-testid="media-block"]');
  await page.locator('img[alt="demo-photo.jpg"]').evaluate((image) => image.decode());
  await page.evaluate(() => document.fonts.ready);
  await page.locator('[data-testid="message-bubble"]').last().scrollIntoViewIfNeeded();

  const metrics = await page.evaluate(() => {
    const bodyText = document.body.textContent ?? "";
    const previewImage = document.querySelector('img[alt="demo-photo.jpg"]');

    return {
      bodyText,
      bodyWidth: document.body.scrollWidth,
      exportButtonCount: document.querySelectorAll('[data-testid="export-button"]').length,
      hasSyntheticPreviewImage: Boolean(previewImage),
      mediaBlockCount: document.querySelectorAll('[data-testid="media-block"]').length,
      searchValue: document.querySelector('[data-testid="search-input"]')?.value ?? null,
      title: document.querySelector('[data-testid="chat-title"]')?.textContent ?? null,
      viewportWidth: window.innerWidth,
    };
  });

  if (metrics.title !== "Design Preview") {
    throw new Error(`Unexpected demo title: ${metrics.title ?? "missing"}`);
  }

  if (hasPrivateDemoText(metrics.bodyText)) {
    throw new Error("README screenshot route contains private-looking text.");
  }

  if (metrics.mediaBlockCount < 2) {
    throw new Error(`Expected at least 2 media blocks, found ${metrics.mediaBlockCount}.`);
  }

  if (!metrics.hasSyntheticPreviewImage) {
    throw new Error("Expected the README screenshot route to render the synthetic image preview.");
  }

  if (metrics.exportButtonCount !== 1) {
    throw new Error(`Expected 1 export button, found ${metrics.exportButtonCount}.`);
  }

  if (metrics.searchValue !== "") {
    throw new Error("Expected an unfiltered screenshot with an empty search box.");
  }

  if (metrics.bodyWidth > metrics.viewportWidth) {
    throw new Error(`Screenshot route overflows horizontally: ${metrics.bodyWidth} > ${metrics.viewportWidth}.`);
  }

  const mediaFits = await page.locator('[data-testid="media-block"]').evaluateAll((blocks) => {
    const canvas = document.querySelector('[data-testid="message-canvas"]').getBoundingClientRect();
    return blocks.every((block) => {
      const box = block.getBoundingClientRect();
      return box.top >= canvas.top && box.bottom <= canvas.bottom
        && box.left >= canvas.left && box.right <= canvas.right;
    });
  });
  if (!mediaFits) {
    throw new Error("README screenshot must show both synthetic media attachments without clipping.");
  }

  await page.screenshot({ path: screenshotPath, fullPage: false, animations: "disabled" });
  console.log(`Captured README screenshot: ${screenshotPath}`);
} finally {
  await browser.close();
}
