import { expect, test } from "@playwright/test";

// Different chats can reuse message and attachment IDs. Their preview state must not leak.
test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const backup = {
      handle: "media-backup", displayName: "Media iPhone", productLabel: "iPhone",
      productVersion: null, lastBackupDate: null, isEncrypted: false,
      hasInfoPlist: true, hasStatusPlist: true, hasManifestPlist: true,
      whatsapp: { manifestReadable: true, hasChatStorage: true, hasContacts: false, mediaFileCount: 2 },
    };
    const chats = ["First", "Second"].map(title => ({
      id: title, title, latestMessage: null, latestMessageTimestamp: null,
      messageCount: 1, attachmentCount: 1,
    }));
    const imported = (title: string) => ({
      source_kind: "iphone_backup", transcript_name: title, issues: [],
      messages: [{ id: "same-message", body: title, sender: "Example",
        timestamp: { raw: "2026-06-23T12:00:00Z" }, attachment_ids: ["same-attachment"] }],
      attachments: [{ id: "same-attachment", archive_path: `${title}.png`, filename: `${title}.png`, kind: "photo", size_bytes: 100,
        ...(title === "First" ? { preview: { mediaType: "image/png", sizeBytes: 100,
          dataUrl: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScLbtAAAAABJRU5ErkJggg==" } } : {}),
      }],
    });
    let finishImport: (value: unknown) => void;
    let failPreview: (reason: string) => void;
    Object.assign(window, {
      __TAURI_INTERNALS__: { invoke(command: string, args?: { chatId?: string }) {
        if (command === "list_iphone_backups") return Promise.resolve([backup]);
        if (command === "list_iphone_backup_chats") return Promise.resolve({ chats, isTruncated: false, limit: 100 });
        if (command === "import_iphone_backup_chat") return args!.chatId === "First"
          ? Promise.resolve(imported("First")) : new Promise(resolve => { finishImport = resolve; });
        if (command === "read_iphone_backup_attachment_preview") return new Promise((_resolve, reject) => { failPreview = reject; });
        return Promise.resolve(null);
      } },
      __finishMediaImport() { finishImport(imported("Second")); },
      __failMediaPreview() { failPreview("Media unavailable"); },
    });
  });
  await page.goto("/");
  await page.getByRole("button", { name: /Media iPhone/ }).click();
  await page.locator(".chat-sidebar").getByRole("button", { name: /First/ }).click();
  await expect(page.getByRole("img", { name: "First.png" })).toBeVisible();
});

declare global {
  interface Window {
    __finishMediaImport(): void;
    __failMediaPreview(): void;
  }
}

test("a new chat cannot display the previous attachment while its preview loads or fails", async ({ page }) => {
  await page.locator(".chat-sidebar").getByRole("button", { name: /Second/ }).click();
  await page.evaluate(() => window.__finishMediaImport());
  await expect(page.getByTestId("chat-title")).toHaveText("Second");
  await expect(page.getByRole("img", { name: "Second.png" })).toHaveCount(0);
  await expect(page.getByText("Loading media")).toBeVisible();
  await page.evaluate(() => window.__failMediaPreview());
  await expect(page.getByRole("img", { name: "Second.png" })).toHaveCount(0);
  await expect(page.getByText("Loading media")).toHaveCount(0);
});

test("an image dialog closes when a pending chat selection completes", async ({ page }) => {
  await page.locator(".chat-sidebar").getByRole("button", { name: /Second/ }).click();
  await page.getByRole("button", { name: "Open First.png" }).focus();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.evaluate(() => window.__finishMediaImport());
  await expect(page.getByTestId("chat-title")).toHaveText("Second");
  await expect(page.getByRole("dialog")).toHaveCount(0);
});
