import { invoke } from "@tauri-apps/api/core";

import { createLoadedBackupSource } from "../domain/source";
import type {
  Attachment,
  AttachmentPreview,
  ChatImport,
  IphoneBackupChatSearchResult,
  HtmlExportResult,
  IphoneBackupChatsResult,
  IphoneBackupCandidate,
  LoadedChatSource,
} from "../models";

export interface OpenLocalChatSourceResult {
  source: LoadedChatSource;
  imported: ChatImport;
}

export function isDesktopRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

async function getCurrentDesktopWindow() {
  if (!isDesktopRuntime()) {
    return null;
  }

  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow();
}

export async function minimizeAppWindow(): Promise<void> {
  const appWindow = await getCurrentDesktopWindow();
  await appWindow?.minimize();
}

export async function toggleMaximizeAppWindow(): Promise<void> {
  const appWindow = await getCurrentDesktopWindow();
  await appWindow?.toggleMaximize();
}

export async function closeAppWindow(): Promise<void> {
  const appWindow = await getCurrentDesktopWindow();
  await appWindow?.close();
}

export async function listIphoneBackups(): Promise<IphoneBackupCandidate[]> {
  if (!isDesktopRuntime()) {
    return [];
  }

  return invoke<IphoneBackupCandidate[]>("list_iphone_backups");
}

export async function chooseIphoneBackupFolder(): Promise<IphoneBackupCandidate[] | null> {
  if (!isDesktopRuntime()) {
    return [];
  }

  return invoke<IphoneBackupCandidate[] | null>("choose_iphone_backup_folder");
}

export async function openLocalChatSource(): Promise<OpenLocalChatSourceResult | null> {
  if (!isDesktopRuntime()) {
    throw new Error("File selection is available in the desktop app.");
  }

  return invoke<OpenLocalChatSourceResult | null>("open_whatsapp_export");
}

export async function listIphoneBackupChats(
  backup: IphoneBackupCandidate,
): Promise<IphoneBackupChatsResult> {
  if (!isDesktopRuntime()) {
    return { chats: [], isTruncated: false, limit: 0 };
  }

  return invoke<IphoneBackupChatsResult>("list_iphone_backup_chats", {
    backupHandle: backup.handle,
  });
}

export async function searchIphoneBackupChats(
  backup: IphoneBackupCandidate,
  query: string,
): Promise<IphoneBackupChatsResult> {
  if (!isDesktopRuntime()) {
    return { chats: [], isTruncated: false, limit: 0 };
  }

  return invoke<IphoneBackupChatsResult>("search_iphone_backup_chats", {
    backupHandle: backup.handle,
    query,
  });
}

export async function importIphoneBackupChat(
  backup: IphoneBackupCandidate,
  chatId: string,
): Promise<OpenLocalChatSourceResult> {
  const imported = await invoke<ChatImport>("import_iphone_backup_chat", {
    backupHandle: backup.handle,
    chatId,
  });

  return {
    source: createLoadedBackupSource(backup, chatId),
    imported,
  };
}

export async function searchIphoneBackupChat(
  source: LoadedChatSource,
  query: string,
): Promise<IphoneBackupChatSearchResult> {
  if (source.kind !== "iphone_backup" || !source.chatId) {
    throw new Error("Open a specific iPhone backup chat before searching the backup.");
  }

  return invoke<IphoneBackupChatSearchResult>("search_iphone_backup_chat", {
    backupHandle: source.handle,
    chatId: source.chatId,
    query,
  });
}

export async function readLocalAttachmentPreview(
  source: LoadedChatSource,
  attachment: Attachment,
): Promise<AttachmentPreview | null> {
  switch (source.kind) {
    case "whatsapp_export_zip":
      return invoke<AttachmentPreview | null>("read_export_attachment_preview", {
        sourceHandle: source.handle,
        archivePath: attachment.archive_path,
      });
    case "iphone_backup":
      return invoke<AttachmentPreview | null>("read_iphone_backup_attachment_preview", {
        backupHandle: source.handle,
        archivePath: attachment.archive_path,
        filename: attachment.filename,
        kind: attachment.kind,
      });
  }
}

export async function exportLocalChatHtml(
  source: LoadedChatSource,
  defaultPath: string,
  title: string,
): Promise<HtmlExportResult | null> {
  switch (source.kind) {
    case "whatsapp_export_zip":
      {
        return invoke<HtmlExportResult | null>("export_whatsapp_export_html", {
          sourceHandle: source.handle,
          defaultFilename: defaultPath,
          title,
        });
      }
    case "iphone_backup":
      {
        if (!source.chatId) {
          throw new Error("Open a specific iPhone backup chat before exporting HTML.");
        }

        return invoke<HtmlExportResult | null>("export_iphone_backup_chat_html", {
          backupHandle: source.handle,
          chatId: source.chatId,
          defaultFilename: defaultPath,
          title,
        });
      }
  }
}
