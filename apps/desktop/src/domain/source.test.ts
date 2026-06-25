import { describe, expect, it } from "vitest";

import {
  backupMetadataLine,
  backupReadiness,
  createLoadedBackupSource,
  createLoadedChatSource,
  sourceProfile,
} from "./source";
import type { IphoneBackupCandidate } from "../models";

function backupCandidate(
  overrides: Partial<IphoneBackupCandidate> = {},
): IphoneBackupCandidate {
  return {
    handle: "backup-source-1",
    displayName: "Example iPhone",
    productLabel: "iPhone 15 Pro",
    productVersion: "18.5",
    lastBackupDate: "2026-06-23T10:00:00Z",
    isEncrypted: false,
    hasInfoPlist: true,
    hasStatusPlist: true,
    hasManifestPlist: true,
    whatsapp: {
      manifestReadable: true,
      hasChatStorage: true,
      hasContacts: true,
      mediaFileCount: 42,
    },
    ...overrides,
  };
}

describe("source domain helpers", () => {
  it("creates source display names without exposing parent paths", () => {
    const source = createLoadedChatSource(
      "whatsapp_export_zip",
      "export-source-1",
      "WhatsApp Chat - Family.zip",
    );

    expect(source).toMatchObject({
      kind: "whatsapp_export_zip",
      handle: "export-source-1",
      displayName: "WhatsApp Chat - Family.zip",
    });
    expect(source).not.toHaveProperty("path");
    expect(source.displayName).toBe("WhatsApp Chat - Family.zip");
    expect(source.displayName).not.toContain("/sample-data");
  });

  it("creates loaded backup sources from safe backup metadata", () => {
    const source = createLoadedBackupSource(backupCandidate(), "chat-1");

    expect(source).toMatchObject({
      kind: "iphone_backup",
      handle: "backup-source-1",
      displayName: "Example iPhone",
      chatId: "chat-1",
    });
    expect(source).not.toHaveProperty("path");
    expect(source.displayName).not.toContain("/sample-data");
  });

  it("centralizes current source labels and picker settings", () => {
    expect(sourceProfile("whatsapp_export_zip")).toMatchObject({
      pickerName: "WhatsApp export ZIP",
      pickerExtensions: ["zip"],
      openActionLabel: "Open WhatsApp export ZIP",
      availabilityLabel: "Available now",
      viewingLabel: "Viewing local export",
      supportsHtmlExport: true,
    });
    expect(sourceProfile("iphone_backup")).toMatchObject({
      pickerName: "iPhone backup",
      availabilityLabel: "Proof work",
      supportsHtmlExport: true,
    });
  });

  it("summarizes backup readiness without exposing local paths", () => {
    expect(backupReadiness(backupCandidate())).toMatchObject({
      tone: "ready",
      label: "WhatsApp found",
      detail: "42 media files mapped",
    });
    expect(
      backupReadiness(
        backupCandidate({
          whatsapp: {
            manifestReadable: true,
            hasChatStorage: false,
            hasContacts: false,
            mediaFileCount: 0,
          },
        }),
      ),
    ).toMatchObject({
      tone: "warning",
      label: "WhatsApp not found",
    });
    expect(backupReadiness(backupCandidate({ isEncrypted: true }))).toMatchObject({
      tone: "blocked",
      label: "Encrypted backup",
    });
  });

  it("builds compact backup metadata lines", () => {
    expect(backupMetadataLine(backupCandidate())).toBe(
      "iPhone 15 Pro · iOS 18.5 · Last backup Jun 23, 2026",
    );
    expect(
      backupMetadataLine(
        backupCandidate({
          productLabel: null,
          productVersion: null,
          lastBackupDate: null,
        }),
      ),
    ).toBe("Local iPhone backup");
    expect(
      backupMetadataLine(
        backupCandidate({
          lastBackupDate: "unknown date",
        }),
      ),
    ).toBe("iPhone 15 Pro · iOS 18.5 · Last backup unknown date");
  });
});
