use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    IphoneBackup,
    WhatsappExportZip,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupCandidate {
    pub id: String,
    pub path: String,
    pub manifest_db_path: String,
    pub manifest_plist_path: Option<String>,
    pub info_plist_path: Option<String>,
    pub status_plist_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BackupMetadata {
    pub device_name: Option<String>,
    pub display_name: Option<String>,
    pub product_name: Option<String>,
    pub product_type: Option<String>,
    pub product_version: Option<String>,
    pub last_backup_date: Option<String>,
    pub is_encrypted: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestFile {
    pub file_id: String,
    pub domain: String,
    pub relative_path: String,
    pub flags: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhatsappManifestFiles {
    pub chat_storage: Option<ManifestFile>,
    pub contacts: Option<ManifestFile>,
    pub media: Vec<ManifestFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatStorageSummary {
    pub message_count: Option<u64>,
    pub chat_count: Option<u64>,
    pub media_item_count: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chat {
    pub id: String,
    pub title: String,
    pub latest_message: Option<String>,
    pub latest_message_timestamp: Option<MessageTimestamp>,
    pub message_count: u64,
    pub attachment_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatImport {
    pub source_kind: SourceKind,
    pub transcript_name: Option<String>,
    pub messages: Vec<Message>,
    pub attachments: Vec<Attachment>,
    pub issues: Vec<ImportIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub timestamp: MessageTimestamp,
    pub sender: Option<String>,
    pub body: String,
    pub attachment_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageTimestamp {
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub archive_path: String,
    pub filename: String,
    pub kind: AttachmentKind,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentKind {
    Audio,
    Gif,
    Photo,
    Sticker,
    Video,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportIssue {
    pub code: ImportIssueCode,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportIssueCode {
    ContinuationWithoutMessage,
    MessageWindowTruncated,
    MultipleTranscripts,
    MissingAttachmentReference,
    NoTranscript,
    SearchResultsTruncated,
}
