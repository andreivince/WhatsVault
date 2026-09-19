import { expect, test } from "@playwright/test";

for (const width of [1440, 390]) {
  test(`chat avatar initials remain centered at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 920 });
    await page.goto("/?demo=backup-chat");
    const offset = await page.locator(".conversation-header .avatar").evaluate((avatar) => {
      const box = avatar.getBoundingClientRect();
      const range = document.createRange();
      range.selectNodeContents(avatar);
      const text = range.getBoundingClientRect();
      return {
        x: Math.abs(text.x + text.width / 2 - box.x - box.width / 2),
        y: Math.abs(text.y + text.height / 2 - box.y - box.height / 2),
      };
    });
    expect(offset.x).toBeLessThanOrEqual(2);
    expect(offset.y).toBeLessThanOrEqual(2);
  });
}

test("app icon speech bubble has a centered silhouette above its tail", async ({ page }) => {
  await page.goto("/");
  const centers = await page.evaluate(async () => {
    const image = new Image();
    image.src = "/app-icon.svg";
    await image.decode();
    const canvas = document.createElement("canvas");
    canvas.width = canvas.height = 1024;
    const context = canvas.getContext("2d")!;
    context.drawImage(image, 0, 0, 1024, 1024);
    return [250, 350, 450, 550, 650].map((y) => {
      const pixels = context.getImageData(0, y, 1024, 1).data;
      const white = Array.from({ length: 1024 }, (_, x) => x)
        .filter((x) => pixels[x * 4] > 250 && pixels[x * 4 + 1] > 250 && pixels[x * 4 + 2] > 250);
      return (white[0] + white[white.length - 1] + 1) / 2;
    });
  });
  for (const center of centers) expect(Math.abs(center - 512)).toBeLessThanOrEqual(2);
});
