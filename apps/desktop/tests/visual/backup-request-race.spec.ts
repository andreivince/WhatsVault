import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
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
    const pendingChatErrors = new Map<string, (reason: unknown) => void>();
    const pendingScans: Array<(value: unknown) => void> = [];
    const pendingScanErrors: Array<(reason: unknown) => void> = [];
    let holdScans = false;

    Object.assign(window, {
      __TAURI_INTERNALS__: {
        invoke(command: string, args?: { backupHandle?: string }) {
          if (command === "list_iphone_backups" || command === "choose_iphone_backup_folder") {
            if (holdScans) {
              return new Promise((resolve, reject) => {
                pendingScans.push(resolve);
                pendingScanErrors.push(reject);
              });
            }
            return Promise.resolve(candidates);
          }

          if (command === "list_iphone_backup_chats" && args?.backupHandle) {
            return new Promise((resolve, reject) => {
              pendingChatLists.set(args.backupHandle!, resolve);
              pendingChatErrors.set(args.backupHandle!, reject);
            });
          }

          throw new Error(`Unexpected Tauri command: ${command}`);
        },
      },
      __holdBackupScans() { holdScans = true; },
      __rejectBackupScan(index: number) { pendingScanErrors[index]("Old scan failed"); },
      __resolveBackupScan(index: number, displayName: string) {
        pendingScans[index]([{ ...candidates[0], handle: `scan-${index}`, displayName }]);
      },
      __rejectBackupChats(backupHandle: string) {
        pendingChatErrors.get(backupHandle)!("Old request failed");
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
  await expect(page.getByRole("button", { name: /First iPhone/ })).toBeVisible();
});

test("a stale backup response cannot replace the latest selected backup chats", async ({ page }) => {
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


declare global {
  interface Window {
    __holdBackupScans(): void;
    __rejectBackupScan(index: number): void;
    __resolveBackupScan(index: number, displayName: string): void;
    __rejectBackupChats(handle: string): void;
    __resolveBackupChats(handle: string, title: string): void;
  }
}

test("a stale backup error cannot clear the newest chat list", async ({ page }) => {
  await page.getByRole("button", { name: /First iPhone/ }).click();
  await page.getByRole("button", { name: /Second iPhone/ }).click();
  await page.evaluate(() => window.__resolveBackupChats("backup-source-2", "Second Chat"));
  await expect(page.getByLabel("Second iPhone chats").getByText("Second Chat")).toBeVisible();
  await page.evaluate(() => window.__rejectBackupChats("backup-source-1"));
  await expect(page.getByLabel("Second iPhone chats").getByText("Second Chat")).toBeVisible();
  await expect(page.getByText("Old request failed")).toHaveCount(0);
});

test("refreshing backups invalidates a pending chat list", async ({ page }) => {
  await page.getByRole("button", { name: /First iPhone/ }).click();
  await page.getByRole("button", { name: "Refresh backups", exact: true }).click();
  await page.evaluate(() => window.__resolveBackupChats("backup-source-1", "Stale Chat"));
  await expect(page.getByText("Stale Chat")).toHaveCount(0);
  await expect(page.getByLabel("First iPhone chats")).toHaveCount(0);
});

test("a late scan cannot replace the newest backup candidates", async ({ page }) => {
  await page.evaluate(() => window.__holdBackupScans());
  await page.getByRole("button", { name: "Refresh backups", exact: true }).click();
  await page.getByRole("button", { name: "Refresh backups", exact: true }).click();
  await page.evaluate(() => window.__resolveBackupScan(1, "Latest iPhone"));
  await expect(page.getByRole("button", { name: /Latest iPhone/ })).toBeVisible();
  await page.evaluate(() => window.__resolveBackupScan(0, "Old iPhone"));
  await expect(page.getByRole("button", { name: /Latest iPhone/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /Old iPhone/ })).toHaveCount(0);
});


test("a stale scan error cannot clear the newest backup candidates", async ({ page }) => {
  await page.evaluate(() => window.__holdBackupScans());
  await page.getByRole("button", { name: "Refresh backups", exact: true }).click();
  await page.getByRole("button", { name: "Refresh backups", exact: true }).click();
  await page.evaluate(() => window.__resolveBackupScan(1, "Latest iPhone"));
  await expect(page.getByRole("button", { name: /Latest iPhone/ })).toBeVisible();
  await page.evaluate(() => window.__rejectBackupScan(0));
  await expect(page.getByRole("button", { name: /Latest iPhone/ })).toBeVisible();
  await expect(page.getByText("Old scan failed")).toHaveCount(0);
});

test("a folder selection supersedes an older automatic scan", async ({ page }) => {
  await page.evaluate(() => window.__holdBackupScans());
  await page.getByRole("button", { name: "Refresh backups", exact: true }).click();
  await page.getByRole("button", { name: "Choose folder", exact: true }).click();
  await page.evaluate(() => window.__resolveBackupScan(1, "Chosen iPhone"));
  await expect(page.getByRole("button", { name: /Chosen iPhone/ })).toBeVisible();
  await page.evaluate(() => window.__resolveBackupChats("scan-1", "Chosen Chat"));
  await expect(page.getByLabel("Chosen iPhone chats").getByText("Chosen Chat")).toBeVisible();
  await page.evaluate(() => window.__resolveBackupScan(0, "Old iPhone"));
  await expect(page.getByLabel("Chosen iPhone chats").getByText("Chosen Chat")).toBeVisible();
  await expect(page.getByRole("button", { name: /Old iPhone/ })).toHaveCount(0);
});
