import { expect, test } from "@playwright/test";

test("a stale backup response cannot replace the latest selected backup chats", async ({ page }) => {
  await page.addInitScript(() => {
    const candidates = ["First iPhone", "Second iPhone"].map((displayName, index) => ({
      handle: `backup-source-${index + 1}`,
      displayName,
      productLabel: "iPhone",
      productVersion: null,
      lastBackupDate: null,
      isEncrypted: false,
      hasInfoPlist: true,
      hasStatusPlist: true,
      hasManifestPlist: true,
      whatsapp: {
        manifestReadable: true,
        hasChatStorage: true,
        hasContacts: false,
        mediaFileCount: 0,
      },
    }));
    const pendingChatLists = new Map<string, (value: unknown) => void>();

    Object.assign(window, {
      __TAURI_INTERNALS__: {
        invoke(command: string, args?: { backupHandle?: string }) {
          if (command === "list_iphone_backups") {
            return Promise.resolve(candidates);
          }

          if (command === "list_iphone_backup_chats" && args?.backupHandle) {
            return new Promise((resolve) => {
              pendingChatLists.set(args.backupHandle!, resolve);
            });
          }

          throw new Error(`Unexpected Tauri command: ${command}`);
        },
      },
      __resolveBackupChats(backupHandle: string, title: string) {
        const resolve = pendingChatLists.get(backupHandle);
        if (!resolve) {
          throw new Error(`No pending chat request for ${backupHandle}`);
        }

        resolve({
          chats: [{
            id: `${backupHandle}-chat`,
            title,
            latestMessage: null,
            latestMessageTimestamp: null,
            messageCount: 1,
            attachmentCount: 0,
          }],
          isTruncated: false,
          limit: 100,
        });
      },
    });
  });

  await page.goto("/");
  await page.getByRole("button", { name: /First iPhone/ }).click();
  await page.getByRole("button", { name: /Second iPhone/ }).click();

  await page.evaluate(() => {
    const runtime = window as typeof window & {
      __resolveBackupChats: (backupHandle: string, title: string) => void;
    };
    runtime.__resolveBackupChats("backup-source-2", "Second Chat");
  });
  await expect(page.getByLabel("Second iPhone chats").getByText("Second Chat")).toBeVisible();

  await page.evaluate(() => {
    const runtime = window as typeof window & {
      __resolveBackupChats: (backupHandle: string, title: string) => void;
    };
    runtime.__resolveBackupChats("backup-source-1", "First Chat");
  });

  await expect(page.getByLabel("Second iPhone chats").getByText("Second Chat")).toBeVisible();
  await expect(page.getByText("First Chat")).toHaveCount(0);
});
