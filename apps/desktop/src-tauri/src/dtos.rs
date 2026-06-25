use serde::Serialize;
use whatsvault_core::{Chat, ChatImport, MessageTimestamp};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedChatSourceDto {
    pub kind: String,
    pub handle: String,
    pub display_name: String,
    pub chat_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenLocalChatSourceResultDto {
    pub source: LoadedChatSourceDto,
    pub imported: ChatImport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentPreviewDto {
    pub media_type: String,
    pub data_url: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HtmlExportResultDto {
    pub embedded_attachment_count: usize,
    pub skipped_attachment_count: usize,
    pub exported_message_count: usize,
    pub skipped_message_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IphoneBackupCandidateDto {
    pub handle: String,
    pub display_name: String,
    pub product_label: Option<String>,
    pub product_version: Option<String>,
    pub last_backup_date: Option<String>,
    pub is_encrypted: Option<bool>,
    pub has_info_plist: bool,
    pub has_status_plist: bool,
    pub has_manifest_plist: bool,
    pub whatsapp: WhatsappBackupStatusDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsappBackupStatusDto {
    pub manifest_readable: bool,
    pub has_chat_storage: bool,
    pub has_contacts: bool,
    pub media_file_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatDto {
    pub id: String,
    pub title: String,
    pub latest_message: Option<String>,
    pub latest_message_timestamp: Option<MessageTimestamp>,
    pub message_count: u64,
    pub attachment_count: u64,
}

impl From<Chat> for ChatDto {
    fn from(chat: Chat) -> Self {
        Self {
            id: chat.id,
            title: chat.title,
            latest_message: chat.latest_message,
            latest_message_timestamp: chat.latest_message_timestamp,
            message_count: chat.message_count,
            attachment_count: chat.attachment_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IphoneBackupChatsResultDto {
    pub chats: Vec<ChatDto>,
    pub is_truncated: bool,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IphoneBackupChatSearchResultDto {
    pub imported: ChatImport,
    pub is_truncated: bool,
    pub limit: usize,
}
