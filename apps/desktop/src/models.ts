export type SourceKind = "iphone_backup" | "whatsapp_export_zip";

export interface LoadedChatSource {
  kind: SourceKind;
  handle: string;
  displayName: string;
  chatId?: string;
}

export interface Chat {
  id: string;
  title: string;
  latestMessage: string | null;
  latestMessageTimestamp: MessageTimestamp | null;
  messageCount: number;
  attachmentCount: number;
}

export interface IphoneBackupCandidate {
  handle: string;
  displayName: string;
  productLabel: string | null;
  productVersion: string | null;
  lastBackupDate: string | null;
  isEncrypted: boolean | null;
  hasInfoPlist: boolean;
  hasStatusPlist: boolean;
  hasManifestPlist: boolean;
  whatsapp: WhatsappBackupStatus;
}

export interface WhatsappBackupStatus {
  manifestReadable: boolean;
  hasChatStorage: boolean;
  hasContacts: boolean;
  mediaFileCount: number;
}

export type AttachmentKind =
  | "audio"
  | "gif"
  | "photo"
  | "sticker"
  | "video"
  | "unknown";

export interface ChatImport {
  source_kind: SourceKind;
  transcript_name: string | null;
  messages: Message[];
  attachments: Attachment[];
  issues: ImportIssue[];
}

export interface Message {
  id: string;
  timestamp: MessageTimestamp;
  sender: string | null;
  body: string;
  attachment_ids: string[];
}

export interface MessageTimestamp {
  raw: string;
}

export interface Attachment {
  id: string;
  archive_path: string;
  filename: string;
  kind: AttachmentKind;
  size_bytes: number;
  preview?: AttachmentPreview;
}

export interface ImportIssue {
  code: string;
  message: string;
}

export interface AttachmentPreview {
  mediaType: string;
  dataUrl: string;
  sizeBytes: number;
}

export interface HtmlExportResult {
  embeddedAttachmentCount: number;
  skippedAttachmentCount: number;
}
