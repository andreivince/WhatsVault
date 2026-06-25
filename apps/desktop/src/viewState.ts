import type { IphoneBackupChatSearchResult, IphoneBackupChatsResult } from "./models";

export type LoadState = "idle" | "loading" | "ready" | "error";
export type BackupScanState = "idle" | "loading" | "ready" | "error";
export type BackupChatState = "idle" | "loading" | "ready" | "error";

export type BackupMessageSearchState = {
  status: BackupChatState;
  query: string;
  result: IphoneBackupChatSearchResult | null;
  message: string | null;
};

export type BackupChatSearchState = {
  status: BackupChatState;
  query: string;
  result: IphoneBackupChatsResult | null;
  message: string | null;
};

export type ConversationBackupSearchStatus = {
  status: BackupChatState;
  message: string | null;
  isTruncated: boolean;
  limit: number;
};

export type BackupChatListSearchStatus = {
  status: BackupChatState;
  message: string | null;
};

export type ExportState = {
  status: "idle" | "exporting" | "success" | "error";
  message: string | null;
};

export type BackupChatListWindow = {
  isTruncated: boolean;
  limit: number;
};
