import { mkdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appDir = dirname(scriptDir);
const repoRoot = resolve(appDir, "../..");
const screenshotPath = join(repoRoot, "docs", "assets", "whatsvault-synthetic-demo.png");
const demoUrl = process.env.WHATSVAULT_DEMO_URL ?? "http://127.0.0.1:1420/?demo=backup-chat";
const privateTextPatternSource = readFileSync(join(appDir, "demo", "private-text-pattern.txt"), "utf8").trim();
const screenshotViewport = { width: 1440, height: 920 };
const messageCanvasScrollTop = 120;

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
  await page.waitForSelector('img[alt="demo-photo.jpg"]');
  await page.locator('[data-testid="message-canvas"]').evaluate((element, scrollTop) => {
    element.scrollTo({ top: scrollTop });
  }, messageCanvasScrollTop);

  const metrics = await page.evaluate((privatePatternSource) => {
    const bodyText = document.body.textContent ?? "";
    const privatePattern = new RegExp(privatePatternSource, "i");
    const previewImage = document.querySelector('img[alt="demo-photo.jpg"]');

    return {
      bodyHasPrivatePattern: privatePattern.test(bodyText),
      bodyWidth: document.body.scrollWidth,
      exportButtonCount: document.querySelectorAll('[data-testid="export-button"]').length,
      hasSyntheticPreviewImage: Boolean(previewImage),
      mediaBlockCount: document.querySelectorAll('[data-testid="media-block"]').length,
      searchValue: document.querySelector('[data-testid="search-input"]')?.value ?? null,
      title: document.querySelector('[data-testid="chat-title"]')?.textContent ?? null,
      viewportWidth: window.innerWidth,
    };
  }, privateTextPatternSource);

  if (metrics.title !== "Design Preview") {
    throw new Error(`Unexpected demo title: ${metrics.title ?? "missing"}`);
  }

  if (metrics.bodyHasPrivatePattern) {
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

  await page.screenshot({ path: screenshotPath, fullPage: false });
  console.log(`Captured README screenshot: ${screenshotPath}`);
} finally {
  await browser.close();
}
