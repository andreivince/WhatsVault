import type { IphoneBackupCandidate, LoadedChatSource, SourceKind } from "../models";

export interface SourceProfile {
  kind: SourceKind;
  displayName: string;
  availabilityLabel: string;
  availabilityDetail: string;
  availabilityTone: "available" | "proof";
  pickerName: string;
  pickerExtensions: string[];
  openActionLabel: string;
  loadingLabel: string;
  emptyLabel: string;
  bannerLabel: string;
  viewingLabel: string;
  supportsHtmlExport: boolean;
}

const SOURCE_PROFILES: Record<SourceKind, SourceProfile> = {
  iphone_backup: {
    kind: "iphone_backup",
    displayName: "iPhone backup",
    availabilityLabel: "Proof work",
    availabilityDetail:
      "Real-backup proof pending. Synthetic backup coverage keeps this path visible while parser work continues.",
    availabilityTone: "proof",
    pickerName: "iPhone backup",
    pickerExtensions: [],
    openActionLabel: "Open iPhone backup",
    loadingLabel: "Opening backup...",
    emptyLabel: "No backup loaded",
    bannerLabel: "Imported iPhone backup",
    viewingLabel: "Viewing local backup",
    supportsHtmlExport: true,
  },
  whatsapp_export_zip: {
    kind: "whatsapp_export_zip",
    displayName: "WhatsApp export ZIP",
    availabilityLabel: "Available now",
    availabilityDetail:
      "ZIP import, search, media preview, and HTML export are available in the desktop app.",
    availabilityTone: "available",
    pickerName: "WhatsApp export ZIP",
    pickerExtensions: ["zip"],
    openActionLabel: "Open WhatsApp export ZIP",
    loadingLabel: "Importing export...",
    emptyLabel: "No local source loaded",
    bannerLabel: "Imported WhatsApp export",
    viewingLabel: "Viewing local export",
    supportsHtmlExport: true,
  },
};

export const DEFAULT_SOURCE_KIND: SourceKind = "whatsapp_export_zip";

export function sourceProfile(kind: SourceKind = DEFAULT_SOURCE_KIND): SourceProfile {
  return SOURCE_PROFILES[kind];
}

export function createLoadedChatSource(
  kind: SourceKind,
  handle: string,
  displayName: string,
): LoadedChatSource {
  return {
    kind,
    handle,
    displayName,
  };
}

export function createLoadedBackupSource(
  backup: IphoneBackupCandidate,
  chatId?: string,
): LoadedChatSource {
  return {
    kind: "iphone_backup",
    handle: backup.handle,
    displayName: backup.displayName,
    chatId,
  };
}

export function createDemoChatSource(): LoadedChatSource {
  return {
    kind: DEFAULT_SOURCE_KIND,
    handle: "demo-export-source",
    displayName: "WhatsApp Chat - Design Preview.zip",
  };
}

export function sourceDisplayName(path: string): string {
  return path.replaceAll("\\", "/").split("/").filter(Boolean).at(-1) ?? path;
}

export type BackupReadinessTone = "ready" | "blocked" | "warning" | "pending";

export interface BackupReadiness {
  tone: BackupReadinessTone;
  label: string;
  detail: string;
}

export function backupReadiness(candidate: IphoneBackupCandidate): BackupReadiness {
  if (candidate.isEncrypted) {
    return {
      tone: "blocked",
      label: "Encrypted backup",
      detail: "Encrypted backups are not supported yet.",
    };
  }

  if (!candidate.whatsapp.manifestReadable) {
    return {
      tone: "blocked",
      label: "Manifest unreadable",
      detail: "WhatsVault found the backup but could not inspect Manifest.db.",
    };
  }

  if (!candidate.whatsapp.hasChatStorage) {
    return {
      tone: "warning",
      label: "WhatsApp not found",
      detail: "The backup is readable, but ChatStorage.sqlite was not found.",
    };
  }

  return {
    tone: "ready",
    label: "WhatsApp found",
    detail:
      candidate.whatsapp.mediaFileCount > 0
        ? `${candidate.whatsapp.mediaFileCount.toLocaleString()} media files mapped`
        : "ChatStorage.sqlite is mapped; media files were not found yet.",
  };
}

export function backupMetadataLine(candidate: IphoneBackupCandidate): string {
  const parts = [
    candidate.productLabel,
    candidate.productVersion ? `iOS ${candidate.productVersion}` : null,
    candidate.lastBackupDate ? `Last backup ${formatBackupDate(candidate.lastBackupDate)}` : null,
  ].filter((part): part is string => Boolean(part));

  return parts.join(" · ") || "Local iPhone backup";
}

function formatBackupDate(value: string): string {
  const date = new Date(value);

  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat("en", {
    day: "numeric",
    month: "short",
    timeZone: "UTC",
    year: "numeric",
  }).format(date);
}
