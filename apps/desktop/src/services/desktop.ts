import { invoke } from "@tauri-apps/api/core";

import { createLoadedBackupSource } from "../domain/source";
import type {
  Attachment,
  AttachmentPreview,
  Chat,
  ChatImport,
  HtmlExportResult,
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

export async function listIphoneBackups(): Promise<IphoneBackupCandidate[]> {
  if (!isDesktopRuntime()) {
    return [];
  }

  return invoke<IphoneBackupCandidate[]>("list_iphone_backups");
}

export async function openLocalChatSource(): Promise<OpenLocalChatSourceResult | null> {
  if (!isDesktopRuntime()) {
    throw new Error("File selection is available in the desktop app.");
  }

  return invoke<OpenLocalChatSourceResult | null>("open_whatsapp_export");
}

export async function listIphoneBackupChats(
  backup: IphoneBackupCandidate,
): Promise<Chat[]> {
  if (!isDesktopRuntime()) {
    return [];
  }

  return invoke<Chat[]>("list_iphone_backup_chats", {
    backupHandle: backup.handle,
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
