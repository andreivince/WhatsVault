import { expect, test, type Page } from "@playwright/test";

import { TEST_IDS } from "../../src/testing/testIds";

const MIN_TEXT_CONTRAST = 4.5;

async function expectFocusRing(page: Page, selector: string) {
  const styles = await page.locator(selector).evaluate((element) => {
    const computed = window.getComputedStyle(element);
    return {
      boxShadow: computed.boxShadow,
      outlineColor: computed.outlineColor,
      outlineStyle: computed.outlineStyle,
      outlineWidth: computed.outlineWidth,
    };
  });

  expect(
    styles.boxShadow !== "none" ||
      (styles.outlineStyle !== "none" && styles.outlineWidth !== "0px"),
  ).toBe(true);
  expect(styles.outlineColor).not.toBe("rgba(0, 0, 0, 0)");
}

function parseRgb(color: string): [number, number, number] {
  const values = color.match(/\d+(\.\d+)?/g)?.slice(0, 3).map(Number);
  if (!values || values.length < 3) {
    throw new Error(`Unsupported color format: ${color}`);
  }

  return [values[0], values[1], values[2]];
}

function relativeLuminance(color: string) {
  const [red, green, blue] = parseRgb(color).map((channel) => {
    const value = channel / 255;
    return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });

  return red * 0.2126 + green * 0.7152 + blue * 0.0722;
}

function contrastRatio(foreground: string, background: string) {
  const lighter = Math.max(relativeLuminance(foreground), relativeLuminance(background));
  const darker = Math.min(relativeLuminance(foreground), relativeLuminance(background));

  return (lighter + 0.05) / (darker + 0.05);
}

test("keyboard focus is visible on primary desktop controls", async ({ page }) => {
  await page.goto("/?demo=backup-chat");

  await page.getByTestId(TEST_IDS.searchInput).focus();
  await expectFocusRing(page, ".search-box");

  await page.getByTestId(TEST_IDS.openSourceButton).focus();
  await expectFocusRing(page, `[data-testid="${TEST_IDS.openSourceButton}"]`);

  await page.getByTestId(TEST_IDS.exportButton).focus();
  await expectFocusRing(page, `[data-testid="${TEST_IDS.exportButton}"]`);

  await page.getByTestId(TEST_IDS.dateFilterInput).focus();
  await expectFocusRing(page, ".date-filter");
});

test("icon-only buttons keep accessible names", async ({ page }) => {
  await page.goto("/?demo=backup-chat");

  const unlabeledButtons = await page.locator("button").evaluateAll((buttons) =>
    buttons
      .filter((button) => {
        const style = window.getComputedStyle(button);
        return style.display !== "none" && style.visibility !== "hidden";
      })
      .filter((button) => button.querySelector("svg") && button.innerText.trim().length === 0)
      .filter((button) => !button.getAttribute("aria-label")?.trim())
      .map((button) => button.className.toString()),
  );

  expect(unlabeledButtons).toEqual([]);
});

test("core text colors meet the app contrast floor", async ({ page }) => {
  await page.goto("/?demo=backups");

  const colorPairs = await page.evaluate(() =>
    [".primary-action", ".backup-status.ready"].map((selector) => {
      const element = document.querySelector(selector);
      if (!element) {
        throw new Error(`Missing contrast target ${selector}`);
      }

      const computed = window.getComputedStyle(element);
      return {
        selector,
        background: computed.backgroundColor,
        foreground: computed.color,
      };
    }),
  );

  for (const pair of colorPairs) {
    expect(contrastRatio(pair.foreground, pair.background), pair.selector).toBeGreaterThanOrEqual(
      MIN_TEXT_CONTRAST,
    );
  }

  await page.goto("/?demo=backup-chat");

  const chatColorPairs = await page.evaluate(() => {
    function visibleBackground(element: Element) {
      let current: Element | null = element;

      while (current) {
        const background = window.getComputedStyle(current).backgroundColor;
        if (!background.endsWith(", 0)") && background !== "transparent" && background !== "rgba(0, 0, 0, 0)") {
          return background;
        }
        current = current.parentElement;
      }

      return window.getComputedStyle(document.body).backgroundColor;
    }

    return [".message-sender", ".chat-row-media", ".banner-state"].map((selector) => {
      const element = document.querySelector(selector);
      if (!element) {
        throw new Error(`Missing contrast target ${selector}`);
      }

      const computed = window.getComputedStyle(element);
      return {
        selector,
        background: visibleBackground(element),
        foreground: computed.color,
      };
    });
  });

  for (const pair of chatColorPairs) {
    expect(contrastRatio(pair.foreground, pair.background), pair.selector).toBeGreaterThanOrEqual(
      MIN_TEXT_CONTRAST,
    );
  }
});
