import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

for (const scenario of ["sources", "chat", "empty search", "image dialog"]) {
  test(`${scenario} has no automatically detectable WCAG A or AA violations`, async ({ page }) => {
    await page.goto(scenario === "sources" ? "/?demo=backups" : "/?demo=backup-chat");
    if (scenario === "sources") {
      await expect(page.getByRole("button", { name: /Demo iPhone/ })).toBeVisible();
    } else {
      await expect(page.getByTestId("chat-title")).toHaveText("Design Preview");
      await expect(page.getByRole("img", { name: "demo-photo.jpg" })).toBeVisible();
    }
    if (scenario === "empty search") {
      await page.getByTestId("search-input").fill("no-matching-demo-message");
      await expect(page.getByText("No messages match these filters.")).toBeVisible();
    }
    if (scenario === "image dialog") {
      await page.getByRole("button", { name: "Open demo-photo.jpg" }).click();
      await expect(page.getByRole("dialog")).toBeVisible();
    }
    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
      .analyze();
    expect(results.violations).toEqual([]);
  });
}
