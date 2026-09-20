import { expect, test, type Page } from "@playwright/test";

// Delayed native-command responses reproduce user actions finishing out of order.
test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const backup = {
      handle: "test-backup", displayName: "Example iPhone", productLabel: "iPhone",
      productVersion: null, lastBackupDate: null, isEncrypted: false,
      hasInfoPlist: true, hasStatusPlist: true, hasManifestPlist: true,
      whatsapp: { manifestReadable: true, hasChatStorage: true, hasContacts: false, mediaFileCount: 0 },
    };
    const chats = ["First", "Second"].map(title => ({
      id: title, title, latestMessage: null, latestMessageTimestamp: null,
      messageCount: 1, attachmentCount: 0,
    }));
    const pending = new Map<string, { resolve: (value: unknown) => void; reject: (reason: string) => void }>();
    const imported = (title: string) => ({
      source_kind: "iphone_backup", transcript_name: title, attachments: [], issues: [],
      messages: [{ id: title, body: `Message from ${title}`, sender: "Example",
        timestamp: { raw: "2026-06-23T12:00:00Z" }, attachment_ids: [] }],
    });
    Object.assign(window, {
      __TAURI_INTERNALS__: {
        invoke(command: string, args?: { chatId?: string }) {
          if (command === "list_iphone_backups") return Promise.resolve([backup]);
          if (command === "list_iphone_backup_chats") return Promise.resolve({ chats, isTruncated: false, limit: 100 });
          const key = command === "import_iphone_backup_chat" ? args!.chatId! : command;
          return new Promise((resolve, reject) => pending.set(key, { resolve, reject }));
        },
      },
      __sessionResolve(key: string) {
        const value = key === "open_whatsapp_export"
          ? { source: { kind: "whatsapp_export_zip", handle: "test-zip", displayName: "ZIP archive" },
            imported: { ...imported("ZIP archive"), source_kind: "whatsapp_export_zip" } }
          : key.startsWith("export_")
            ? { embeddedAttachmentCount: 42, skippedAttachmentCount: 0, exportedMessageCount: 1, skippedMessageCount: 0 }
            : imported(key);
        pending.get(key)!.resolve(value);
      },
      __sessionReject(key: string) { pending.get(key)!.reject("Could not open this source."); },
    });
  });
  await page.goto("/");
  await page.getByRole("button", { name: /Example iPhone/ }).click();
  await expect(page.locator(".chat-sidebar").getByRole("button", { name: /First/ })).toBeVisible();
});

declare global {
  interface Window {
    __sessionResolve(key: string): void;
    __sessionReject(key: string): void;
  }
}

const chatRow = (page: Page, name: string) => page.locator(".chat-sidebar").getByRole("button", { name: new RegExp(name) });
async function openChat(page: Page, title: string) {
  await chatRow(page, title).click();
  await page.evaluate(key => window.__sessionResolve(key), title);
  await expect(page.getByTestId("chat-title")).toHaveText(title);
}

test("the latest chat stays open when an earlier import finishes last", async ({ page }) => {
  await chatRow(page, "First").click();
  await openChat(page, "Second");
  await page.evaluate(() => window.__sessionResolve("First"));
  await expect(page.getByTestId("chat-title")).toHaveText("Second");
});

test("an old import finishing cannot clear the newer chat loading state", async ({ page }) => {
  await chatRow(page, "First").click();
  await chatRow(page, "Second").click();
  await page.evaluate(() => window.__sessionResolve("First"));
  await expect(chatRow(page, "Second")).toBeDisabled();
  await page.evaluate(() => window.__sessionResolve("Second"));
  await expect(page.getByTestId("chat-title")).toHaveText("Second");
});

test("a failed import is visible without losing the currently open conversation", async ({ page }) => {
  await openChat(page, "First");
  await chatRow(page, "Second").click();
  await page.evaluate(() => window.__sessionReject("Second"));
  await expect(page.getByRole("alert")).toContainText("Could not open this source.");
  await expect(page.getByTestId("chat-title")).toHaveText("First");
});

test("an export from an old conversation cannot post a success notice on a new one", async ({ page }) => {
  await openChat(page, "First");
  await page.getByRole("button", { name: "Export chat to HTML", exact: true }).click();
  await openChat(page, "Second");
  await page.evaluate(() => window.__sessionResolve("export_iphone_backup_chat_html"));
  await expect(page.getByText("42 media files embedded")).toHaveCount(0);
});

test("an old ZIP import cannot replace a more recently selected backup chat", async ({ page }) => {
  await page.getByTestId("open-source-button").click();
  await openChat(page, "Second");
  await page.evaluate(() => window.__sessionResolve("open_whatsapp_export"));
  await expect(page.getByTestId("chat-title")).toHaveText("Second");
});

test("returning to sources cancels a pending import and permits another backup selection", async ({ page }) => {
  await openChat(page, "First");
  await chatRow(page, "Second").click();
  await page.getByRole("button", { name: "Change source", exact: true }).click();
  await expect(page.getByRole("region", { name: "Detected iPhone backups" })).toBeVisible();
  await page.evaluate(() => window.__sessionResolve("Second"));
  await expect(page.getByTestId("chat-title")).toHaveCount(0);
  await openChat(page, "First");
});

test("a stale import error cannot interrupt the newest conversation", async ({ page }) => {
  await chatRow(page, "First").click();
  await openChat(page, "Second");
  await page.evaluate(() => window.__sessionReject("First"));
  await expect(page.getByTestId("chat-title")).toHaveText("Second");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Export chat to HTML", exact: true })).toBeEnabled();
});

test("an export error from an old conversation is ignored after switching chats", async ({ page }) => {
  await openChat(page, "First");
  await page.getByRole("button", { name: "Export chat to HTML", exact: true }).click();
  await openChat(page, "Second");
  await page.evaluate(() => window.__sessionReject("export_iphone_backup_chat_html"));
  await expect(page.getByText("Could not open this source.")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Export chat to HTML", exact: true })).toBeEnabled();
});

test("only one ZIP picker can remain active while switching views", async ({ page }) => {
  await page.getByTestId("open-source-button").click();
  await openChat(page, "First");
  await expect(page.getByTestId("open-source-button")).toBeDisabled();
  await page.getByRole("button", { name: "Change source", exact: true }).click();
  await expect(page.getByTestId("supported-source-card").getByRole("button")).toBeDisabled();
  await page.evaluate(() => window.__sessionResolve("open_whatsapp_export"));
  await expect(page.getByTestId("supported-source-card").getByRole("button")).toBeEnabled();
  await expect(page.getByTestId("chat-title")).toHaveCount(0);
});
