use std::{
    collections::HashMap,
    fs,
    fs::File,
    path::{Path, PathBuf},
    sync::Mutex,
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::{DialogExt, FilePath};
use whatsvault_core::{
    exports::html::{build_chat_html_export, EmbeddedAttachment, HtmlExportOptions},
    media::attachment_media_type,
    sources::iphone_backup::{
        discover_default_backup_candidates, find_whatsapp_manifest_files,
        physical_backup_file_path, read_backup_metadata, resolve_whatsapp_media_file_path,
    },
    sources::whatsapp_export_zip::{import_whatsapp_export_zip, read_whatsapp_export_attachment},
    whatsapp::chat_storage::{import_chat_storage_chat, list_chat_storage_chats},
    AttachmentKind, BackupCandidate, BackupMetadata, Chat, ChatImport, WhatsappManifestFiles,
};

const ATTACHMENT_PREVIEW_MAX_BYTES: u64 = 8 * 1024 * 1024;
const ATTACHMENT_EXPORT_MAX_BYTES: u64 = 24 * 1024 * 1024;
const TOTAL_EXPORT_EMBEDDED_MEDIA_MAX_BYTES: u64 = 128 * 1024 * 1024;
type SourceRegistryState = Mutex<SourceRegistry>;

#[derive(Debug, Default)]
struct SourceRegistry {
    backup_paths: HashMap<String, PathBuf>,
    export_paths: HashMap<String, PathBuf>,
    next_export_handle: u64,
}

impl SourceRegistry {
    fn clear_backups(&mut self) {
        self.backup_paths.clear();
    }

    fn register_backup(&mut self, index: usize, path: PathBuf) -> String {
        let handle = format!("backup-source-{}", index + 1);
        self.backup_paths.insert(handle.clone(), path);
        handle
    }

    fn register_export(&mut self, path: PathBuf) -> String {
        self.next_export_handle += 1;
        let handle = format!("export-source-{}", self.next_export_handle);
        self.export_paths.insert(handle.clone(), path);
        handle
    }

    fn backup_path(&self, handle: &str) -> Option<PathBuf> {
        self.backup_paths.get(handle).cloned()
    }

    fn export_path(&self, handle: &str) -> Option<PathBuf> {
        self.export_paths.get(handle).cloned()
    }
}

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

#[tauri::command]
fn list_iphone_backups(
    registry: State<'_, SourceRegistryState>,
) -> Result<Vec<IphoneBackupCandidateDto>, String> {
    let candidates = discover_default_backup_candidates().map_err(|_| {
        "Could not scan the default iPhone backup folders on this computer.".to_owned()
    })?;
    let mut registry = registry
        .lock()
        .map_err(|_| "Could not access local source handles.".to_owned())?;
    registry.clear_backups();

    Ok(candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let handle = registry.register_backup(index, PathBuf::from(&candidate.path));
            backup_candidate_dto(candidate, index, handle)
        })
        .collect())
}

#[tauri::command]
async fn open_whatsapp_export(
    app: AppHandle,
    registry: State<'_, SourceRegistryState>,
) -> Result<Option<OpenLocalChatSourceResultDto>, String> {
    let Some(source_path) = select_whatsapp_export_path(&app)? else {
        return Ok(None);
    };

    let file = File::open(&source_path)
        .map_err(|error| PublicError::SelectedSourceUnreadable.redact(error))?;
    let imported = import_whatsapp_export_zip(file)
        .map_err(|error| PublicError::SelectedSourceImportFailed.redact(error))?;
    let display_name = source_display_name(&source_path);
    let handle = registry
        .lock()
        .map_err(|_| "Could not access local source handles.".to_owned())?
        .register_export(source_path);

    Ok(Some(OpenLocalChatSourceResultDto {
        source: LoadedChatSourceDto {
            kind: "whatsapp_export_zip".to_owned(),
            handle,
            display_name,
            chat_id: None,
        },
        imported,
    }))
}

#[tauri::command]
fn list_iphone_backup_chats(
    backup_handle: String,
    registry: State<'_, SourceRegistryState>,
) -> Result<Vec<Chat>, String> {
    let backup_path = registered_backup_path(&registry, &backup_handle)?;
    list_iphone_backup_chats_from_path(&backup_path)
}

fn list_iphone_backup_chats_from_path(backup_path: &Path) -> Result<Vec<Chat>, String> {
    let chat_storage_path = resolved_chat_storage_path(backup_path)?;
    list_chat_storage_chats(chat_storage_path)
        .map_err(|error| PublicError::BackupChatListFailed.redact(error))
}

#[tauri::command]
fn import_iphone_backup_chat(
    backup_handle: String,
    chat_id: String,
    registry: State<'_, SourceRegistryState>,
) -> Result<ChatImport, String> {
    let backup_path = registered_backup_path(&registry, &backup_handle)?;
    import_iphone_backup_chat_from_path(&backup_path, &chat_id)
}

fn import_iphone_backup_chat_from_path(
    backup_path: &Path,
    chat_id: &str,
) -> Result<ChatImport, String> {
    let chat_storage_path = resolved_chat_storage_path(backup_path)?;
    import_chat_storage_chat(chat_storage_path, chat_id)
        .map_err(|error| PublicError::BackupChatImportFailed.redact(error))
}

#[tauri::command]
fn read_export_attachment_preview(
    source_handle: String,
    archive_path: String,
    registry: State<'_, SourceRegistryState>,
) -> Result<Option<AttachmentPreviewDto>, String> {
    let source_path = registered_export_path(&registry, &source_handle)?;
    let file = File::open(source_path)
        .map_err(|error| PublicError::SelectedSourceUnreadable.redact(error))?;
    let Some(payload) =
        read_whatsapp_export_attachment(file, &archive_path, ATTACHMENT_PREVIEW_MAX_BYTES)
            .map_err(|error| PublicError::AttachmentPreviewFailed.redact(error))?
    else {
        return Ok(None);
    };
    let Some(media_type) = attachment_media_type(payload.kind, &payload.filename) else {
        return Ok(None);
    };

    Ok(Some(attachment_preview_dto(
        media_type,
        payload.bytes,
        payload.size_bytes,
    )))
}

#[tauri::command]
fn read_iphone_backup_attachment_preview(
    backup_handle: String,
    archive_path: String,
    filename: String,
    kind: AttachmentKind,
    registry: State<'_, SourceRegistryState>,
) -> Result<Option<AttachmentPreviewDto>, String> {
    let backup_path = registered_backup_path(&registry, &backup_handle)?;
    read_iphone_backup_attachment_preview_from_path(&backup_path, &archive_path, &filename, kind)
}

fn read_iphone_backup_attachment_preview_from_path(
    backup_path: &Path,
    archive_path: &str,
    filename: &str,
    kind: AttachmentKind,
) -> Result<Option<AttachmentPreviewDto>, String> {
    let Some(media_type) = attachment_media_type(kind, filename) else {
        return Ok(None);
    };
    let Some((bytes, size_bytes)) = read_iphone_backup_attachment_bytes(
        backup_path,
        archive_path,
        ATTACHMENT_PREVIEW_MAX_BYTES,
    )?
    else {
        return Ok(None);
    };

    Ok(Some(attachment_preview_dto(media_type, bytes, size_bytes)))
}

#[tauri::command]
async fn export_whatsapp_export_html(
    app: AppHandle,
    source_handle: String,
    default_filename: String,
    title: String,
    registry: State<'_, SourceRegistryState>,
) -> Result<Option<HtmlExportResultDto>, String> {
    let source_path = registered_export_path(&registry, &source_handle)?;
    let Some(output_path) = select_html_export_path(&app, &default_filename)? else {
        return Ok(None);
    };

    export_whatsapp_export_html_file(&source_path, &output_path, &title).map(Some)
}

fn export_whatsapp_export_html_file(
    source_path: &Path,
    output_path: &Path,
    title: &str,
) -> Result<HtmlExportResultDto, String> {
    let source_file = File::open(source_path)
        .map_err(|error| PublicError::SelectedSourceUnreadable.redact(error))?;
    let imported = import_whatsapp_export_zip(source_file)
        .map_err(|error| PublicError::SelectedSourceImportFailed.redact(error))?;
    let mut embedded_attachments = Vec::new();
    let mut embedded_media_bytes = 0_u64;

    for attachment in &imported.attachments {
        if attachment.size_bytes > ATTACHMENT_EXPORT_MAX_BYTES {
            continue;
        }
        if embedded_media_bytes.saturating_add(attachment.size_bytes)
            > TOTAL_EXPORT_EMBEDDED_MEDIA_MAX_BYTES
        {
            continue;
        }

        let Some(payload) = read_whatsapp_export_attachment(
            File::open(source_path)
                .map_err(|error| PublicError::SelectedSourceUnreadable.redact(error))?,
            &attachment.archive_path,
            ATTACHMENT_EXPORT_MAX_BYTES,
        )
        .map_err(|error| PublicError::HtmlExportFailed.redact(error))?
        else {
            continue;
        };
        let Some(media_type) = attachment_media_type(payload.kind, &payload.filename) else {
            continue;
        };

        embedded_attachments.push(EmbeddedAttachment {
            attachment_id: attachment.id.clone(),
            media_type: media_type.to_owned(),
            base64_data: STANDARD.encode(payload.bytes),
        });
        embedded_media_bytes = embedded_media_bytes.saturating_add(payload.size_bytes);
    }

    write_chat_html_export(&imported, output_path, title, embedded_attachments)
}

#[tauri::command]
async fn export_iphone_backup_chat_html(
    app: AppHandle,
    backup_handle: String,
    chat_id: String,
    default_filename: String,
    title: String,
    registry: State<'_, SourceRegistryState>,
) -> Result<Option<HtmlExportResultDto>, String> {
    let backup_path = registered_backup_path(&registry, &backup_handle)?;
    let Some(output_path) = select_html_export_path(&app, &default_filename)? else {
        return Ok(None);
    };

    export_iphone_backup_chat_html_file(&backup_path, &chat_id, &output_path, &title).map(Some)
}

fn export_iphone_backup_chat_html_file(
    backup_path: &Path,
    chat_id: &str,
    output_path: &Path,
    title: &str,
) -> Result<HtmlExportResultDto, String> {
    let chat_storage_path = resolved_chat_storage_path(backup_path)?;
    let imported = import_chat_storage_chat(chat_storage_path, chat_id)
        .map_err(|error| PublicError::BackupChatImportFailed.redact(error))?;
    let mut embedded_attachments = Vec::new();
    let mut embedded_media_bytes = 0_u64;

    for attachment in &imported.attachments {
        if attachment.size_bytes > ATTACHMENT_EXPORT_MAX_BYTES {
            continue;
        }
        if embedded_media_bytes.saturating_add(attachment.size_bytes)
            > TOTAL_EXPORT_EMBEDDED_MEDIA_MAX_BYTES
        {
            continue;
        }

        let Some((bytes, size_bytes)) = read_iphone_backup_attachment_bytes(
            backup_path,
            &attachment.archive_path,
            ATTACHMENT_EXPORT_MAX_BYTES,
        )?
        else {
            continue;
        };
        let Some(media_type) = attachment_media_type(attachment.kind, &attachment.filename) else {
            continue;
        };

        embedded_attachments.push(EmbeddedAttachment {
            attachment_id: attachment.id.clone(),
            media_type: media_type.to_owned(),
            base64_data: STANDARD.encode(bytes),
        });
        embedded_media_bytes = embedded_media_bytes.saturating_add(size_bytes);
    }

    write_chat_html_export(&imported, output_path, title, embedded_attachments)
}

pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(SourceRegistry::default()))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_iphone_backups,
            open_whatsapp_export,
            list_iphone_backup_chats,
            import_iphone_backup_chat,
            read_export_attachment_preview,
            read_iphone_backup_attachment_preview,
            export_whatsapp_export_html,
            export_iphone_backup_chat_html
        ])
        .run(tauri::generate_context!())
        .expect("failed to run WhatsVault desktop app");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PublicError {
    SelectedSourceUnreadable,
    SelectedSourceImportFailed,
    AttachmentPreviewFailed,
    BackupChatListFailed,
    BackupChatImportFailed,
    HtmlExportFailed,
    ExportLocationUnavailable,
}

impl PublicError {
    fn message(self) -> &'static str {
        match self {
            Self::SelectedSourceUnreadable => "Could not read the selected local source.",
            Self::SelectedSourceImportFailed => "Could not import the selected local source.",
            Self::AttachmentPreviewFailed => "Could not read media from the selected local source.",
            Self::BackupChatListFailed => "Could not read chats from the selected iPhone backup.",
            Self::BackupChatImportFailed => "Could not open the selected iPhone backup chat.",
            Self::HtmlExportFailed => "Could not write the HTML export.",
            Self::ExportLocationUnavailable => "Could not use the selected export location.",
        }
    }

    fn redact(self, _error: impl std::fmt::Display) -> String {
        self.message().to_owned()
    }
}

fn registered_backup_path(
    registry: &SourceRegistryState,
    backup_handle: &str,
) -> Result<PathBuf, String> {
    registry
        .lock()
        .map_err(|_| "Could not access local source handles.".to_owned())?
        .backup_path(backup_handle)
        .ok_or_else(|| {
            "The selected iPhone backup is no longer available. Refresh backups and try again."
                .to_owned()
        })
}

fn registered_export_path(
    registry: &SourceRegistryState,
    source_handle: &str,
) -> Result<PathBuf, String> {
    registry
        .lock()
        .map_err(|_| "Could not access local source handles.".to_owned())?
        .export_path(source_handle)
        .ok_or_else(|| {
            "The selected WhatsApp export is no longer available. Open it again and try again."
                .to_owned()
        })
}

fn write_chat_html_export(
    imported: &ChatImport,
    output_path: &Path,
    title: &str,
    embedded_attachments: Vec<EmbeddedAttachment>,
) -> Result<HtmlExportResultDto, String> {
    let embedded_attachment_count = embedded_attachments.len();
    let html = build_chat_html_export(
        imported,
        &HtmlExportOptions {
            title: title.to_owned(),
        },
        &embedded_attachments,
    );
    fs::write(output_path, html).map_err(|error| PublicError::HtmlExportFailed.redact(error))?;

    Ok(HtmlExportResultDto {
        embedded_attachment_count,
        skipped_attachment_count: imported
            .attachments
            .len()
            .saturating_sub(embedded_attachment_count),
    })
}

fn select_whatsapp_export_path(app: &AppHandle) -> Result<Option<PathBuf>, String> {
    app.dialog()
        .file()
        .add_filter("WhatsApp export ZIP", &["zip"])
        .set_title("Open WhatsApp export ZIP")
        .blocking_pick_file()
        .map(file_path_to_path_buf)
        .transpose()
}

fn select_html_export_path(
    app: &AppHandle,
    default_filename: &str,
) -> Result<Option<PathBuf>, String> {
    app.dialog()
        .file()
        .add_filter("HTML document", &["html"])
        .set_file_name(safe_html_default_filename(default_filename))
        .set_title("Export chat to HTML")
        .blocking_save_file()
        .map(file_path_to_path_buf)
        .transpose()
}

fn file_path_to_path_buf(file_path: FilePath) -> Result<PathBuf, String> {
    PathBuf::try_from(file_path)
        .map_err(|error| PublicError::ExportLocationUnavailable.redact(error))
}

fn safe_html_default_filename(default_filename: &str) -> String {
    let normalized = default_filename.replace('\\', "/");
    let leaf = normalized.rsplit('/').next().unwrap_or_default().trim();
    let mut cleaned = leaf
        .chars()
        .map(|character| match character {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '-' | '_' => character,
            _ => '-',
        })
        .collect::<String>();

    while cleaned.contains("--") {
        cleaned = cleaned.replace("--", "-");
    }

    let cleaned = cleaned
        .trim_matches(|character| character == '-' || character == '.' || character == '_')
        .chars()
        .take(120)
        .collect::<String>();
    let stem = cleaned
        .strip_suffix(".html")
        .or_else(|| cleaned.strip_suffix(".HTML"))
        .unwrap_or(&cleaned)
        .trim_matches(|character| character == '-' || character == '.' || character == '_');

    if stem.is_empty() {
        "whatsvault-chat.html".to_owned()
    } else {
        format!("{stem}.html")
    }
}

fn attachment_preview_dto(
    media_type: &'static str,
    bytes: Vec<u8>,
    size_bytes: u64,
) -> AttachmentPreviewDto {
    let encoded = STANDARD.encode(bytes);

    AttachmentPreviewDto {
        media_type: media_type.to_owned(),
        data_url: format!("data:{media_type};base64,{encoded}"),
        size_bytes,
    }
}

fn read_iphone_backup_attachment_bytes(
    backup_path: &Path,
    archive_path: &str,
    max_size_bytes: u64,
) -> Result<Option<(Vec<u8>, u64)>, String> {
    let backup_root = backup_path;
    let manifest_db_path = backup_root.join("Manifest.db");
    let Some(media_path) =
        resolve_whatsapp_media_file_path(backup_root, &manifest_db_path, archive_path)
            .map_err(|_| "Could not resolve media from the selected iPhone backup.".to_owned())?
    else {
        return Ok(None);
    };
    let Ok(metadata) = fs::metadata(&media_path) else {
        return Ok(None);
    };
    if !metadata.is_file() || metadata.len() > max_size_bytes {
        return Ok(None);
    }

    let Ok(bytes) = fs::read(&media_path) else {
        return Ok(None);
    };
    let size_bytes = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if size_bytes > max_size_bytes {
        return Ok(None);
    }

    Ok(Some((bytes, size_bytes)))
}

fn resolved_chat_storage_path(backup_path: &Path) -> Result<PathBuf, String> {
    let backup_root = backup_path;
    let manifest_db_path = backup_root.join("Manifest.db");
    let whatsapp = find_whatsapp_manifest_files(&manifest_db_path)
        .map_err(|_| "Could not inspect the selected iPhone backup manifest.".to_owned())?;
    let chat_storage = whatsapp
        .chat_storage
        .as_ref()
        .ok_or_else(|| "WhatsApp ChatStorage.sqlite was not found in this backup.".to_owned())?;
    let chat_storage_path = physical_backup_file_path(backup_root, &chat_storage.file_id);

    if !chat_storage_path.is_file() {
        return Err(
            "WhatsApp ChatStorage.sqlite is mapped but not readable in this backup.".to_owned(),
        );
    }

    Ok(chat_storage_path)
}

fn backup_candidate_dto(
    candidate: &BackupCandidate,
    index: usize,
    handle: String,
) -> IphoneBackupCandidateDto {
    let metadata = read_backup_metadata(candidate).unwrap_or_default();
    let whatsapp = find_whatsapp_manifest_files(&candidate.manifest_db_path)
        .map(whatsapp_backup_status)
        .unwrap_or_else(|_| WhatsappBackupStatusDto {
            manifest_readable: false,
            has_chat_storage: false,
            has_contacts: false,
            media_file_count: 0,
        });

    IphoneBackupCandidateDto {
        handle,
        display_name: backup_display_name(&metadata, index),
        product_label: backup_product_label(&metadata),
        product_version: metadata.product_version,
        last_backup_date: metadata.last_backup_date,
        is_encrypted: metadata.is_encrypted,
        has_info_plist: candidate.info_plist_path.is_some(),
        has_status_plist: candidate.status_plist_path.is_some(),
        has_manifest_plist: candidate.manifest_plist_path.is_some(),
        whatsapp,
    }
}

fn source_display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "Selected WhatsApp export".to_owned())
}

fn whatsapp_backup_status(files: WhatsappManifestFiles) -> WhatsappBackupStatusDto {
    WhatsappBackupStatusDto {
        manifest_readable: true,
        has_chat_storage: files.chat_storage.is_some(),
        has_contacts: files.contacts.is_some(),
        media_file_count: files.media.len(),
    }
}

fn backup_display_name(metadata: &BackupMetadata, index: usize) -> String {
    metadata
        .display_name
        .as_deref()
        .or(metadata.device_name.as_deref())
        .or(metadata.product_name.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("iPhone backup {}", index + 1))
}

fn backup_product_label(metadata: &BackupMetadata) -> Option<String> {
    match (
        metadata.product_name.as_deref(),
        metadata.product_type.as_deref(),
    ) {
        (Some(name), Some(product_type)) if name != product_type => {
            Some(format!("{name} · {product_type}"))
        }
        (Some(name), _) => Some(name.to_owned()),
        (_, Some(product_type)) => Some(product_type.to_owned()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write, path::Path};

    use tempfile::tempdir;
    use whatsvault_core::{media::attachment_media_type, AttachmentKind};
    use zip::write::SimpleFileOptions;

    use super::{
        backup_candidate_dto, backup_display_name, backup_product_label,
        export_iphone_backup_chat_html_file, export_whatsapp_export_html_file,
        import_iphone_backup_chat_from_path, list_iphone_backup_chats_from_path,
        read_iphone_backup_attachment_preview_from_path, safe_html_default_filename,
        source_display_name, PublicError, SourceRegistry,
    };
    use whatsvault_core::{BackupCandidate, BackupMetadata};

    #[test]
    fn maps_previewable_export_media_to_browser_media_types() {
        assert_eq!(
            attachment_media_type(AttachmentKind::Photo, "photo.JPG"),
            Some("image/jpeg")
        );
        assert_eq!(
            attachment_media_type(AttachmentKind::Sticker, "sticker.webp"),
            Some("image/webp")
        );
        assert_eq!(
            attachment_media_type(AttachmentKind::Gif, "animated.mp4"),
            Some("image/gif")
        );
        assert_eq!(
            attachment_media_type(AttachmentKind::Video, "clip.mp4"),
            Some("video/mp4")
        );
    }

    #[test]
    fn backup_display_helpers_prefer_metadata_without_paths_or_ids() {
        let metadata = BackupMetadata {
            device_name: Some("Example iPhone".to_owned()),
            product_name: Some("iPhone 15 Pro".to_owned()),
            product_type: Some("iPhone16,1".to_owned()),
            ..BackupMetadata::default()
        };

        assert_eq!(backup_display_name(&metadata, 4), "Example iPhone");
        assert_eq!(
            backup_product_label(&metadata).as_deref(),
            Some("iPhone 15 Pro · iPhone16,1")
        );
        assert_eq!(
            backup_display_name(&BackupMetadata::default(), 2),
            "iPhone backup 3"
        );
    }

    #[test]
    fn backup_candidate_dto_hides_raw_identifier_from_display_name() {
        let root = tempdir().unwrap();
        let backup_path = root.path().join("synthetic-device-backup-id");
        fs::create_dir_all(&backup_path).unwrap();
        let manifest_db_path = backup_path.join("Manifest.db");
        rusqlite::Connection::open(&manifest_db_path)
            .unwrap()
            .execute_batch(
                r#"
                CREATE TABLE Files (
                    fileID TEXT PRIMARY KEY,
                    domain TEXT,
                    relativePath TEXT,
                    flags INTEGER,
                    file BLOB
                );
                "#,
            )
            .unwrap();

        let candidate = BackupCandidate {
            id: "synthetic-device-backup-id".to_owned(),
            path: backup_path.to_string_lossy().into_owned(),
            manifest_db_path: manifest_db_path.to_string_lossy().into_owned(),
            manifest_plist_path: None,
            info_plist_path: None,
            status_plist_path: None,
        };

        let dto = backup_candidate_dto(&candidate, 0, "backup-source-1".to_owned());

        assert_eq!(dto.handle, "backup-source-1");
        assert_eq!(dto.display_name, "iPhone backup 1");
        assert!(!dto.display_name.contains("synthetic-device-backup-id"));
        assert!(dto.whatsapp.manifest_readable);
    }

    #[test]
    fn source_registry_returns_opaque_handles_for_private_paths() {
        let mut registry = SourceRegistry::default();
        let backup_path =
            Path::new("/Users/example/Library/Application Support/MobileSync/Backup/private-id");
        let export_path = Path::new("/Users/example/Downloads/WhatsApp Chat - Family.zip");

        let backup_handle = registry.register_backup(0, backup_path.to_path_buf());
        let export_handle = registry.register_export(export_path.to_path_buf());

        assert_eq!(backup_handle, "backup-source-1");
        assert_eq!(export_handle, "export-source-1");
        assert!(!backup_handle.contains("Users"));
        assert!(!export_handle.contains("WhatsApp Chat"));
        assert_eq!(
            registry.backup_path(&backup_handle).as_deref(),
            Some(backup_path)
        );
        assert_eq!(
            registry.export_path(&export_handle).as_deref(),
            Some(export_path)
        );
    }

    #[test]
    fn source_display_name_uses_only_the_selected_filename() {
        let display_name = source_display_name(Path::new(
            "/Users/example/Downloads/WhatsApp Chat - Family.zip",
        ));

        assert_eq!(display_name, "WhatsApp Chat - Family.zip");
        assert!(!display_name.contains("/Users/example"));
    }

    #[test]
    fn html_export_defaults_are_sanitized_filenames_not_paths() {
        assert_eq!(
            safe_html_default_filename("/Users/example/Desktop/Family Chat.html"),
            "Family-Chat.html"
        );
        assert_eq!(
            safe_html_default_filename(" ../bad <name> "),
            "bad-name.html"
        );
        assert_eq!(safe_html_default_filename("🔥"), "whatsvault-chat.html");
    }

    #[test]
    fn public_errors_do_not_expose_private_paths() {
        let raw_error = std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "/Users/example/Library/Application Support/MobileSync/Backup/private-id/Manifest.db",
        );
        let message = PublicError::HtmlExportFailed.redact(raw_error);

        assert_eq!(message, "Could not write the HTML export.");
        assert!(!message.contains("/Users/example"));
        assert!(!message.contains("private-id"));
        assert!(!message.contains("Manifest.db"));
    }

    #[test]
    fn lists_iphone_backup_chats_from_resolved_chat_storage() {
        let root = tempdir().unwrap();
        let backup_path = create_synthetic_backup_with_chat_storage(root.path());

        let chats = list_iphone_backup_chats_from_path(&backup_path).unwrap();

        assert_eq!(chats.len(), 1);
        assert_eq!(chats[0].id, "1");
        assert_eq!(chats[0].title, "Backup Chat");
        assert_eq!(chats[0].message_count, 2);
    }

    #[test]
    fn imports_iphone_backup_chat_from_resolved_chat_storage() {
        let root = tempdir().unwrap();
        let backup_path = create_synthetic_backup_with_chat_storage(root.path());

        let imported = import_iphone_backup_chat_from_path(&backup_path, "1").unwrap();

        assert_eq!(imported.transcript_name.as_deref(), Some("Backup Chat"));
        assert_eq!(imported.messages.len(), 2);
        assert_eq!(imported.messages[0].body, "hello from backup");
        assert_eq!(imported.messages[1].sender.as_deref(), Some("You"));
    }

    #[test]
    fn reads_iphone_backup_attachment_preview_from_resolved_media_file() {
        let root = tempdir().unwrap();
        let backup_path = create_synthetic_backup_with_media(root.path());

        let preview = read_iphone_backup_attachment_preview_from_path(
            &backup_path,
            "Message/Media/photo.jpg",
            "photo.jpg",
            AttachmentKind::Photo,
        )
        .unwrap()
        .unwrap();

        assert_eq!(preview.media_type, "image/jpeg");
        assert_eq!(preview.size_bytes, 17);
        assert_eq!(
            preview.data_url,
            "data:image/jpeg;base64,ZmFrZSBiYWNrdXAgaW1hZ2U="
        );
    }

    #[test]
    fn returns_empty_iphone_backup_preview_for_missing_manifest_media() {
        let root = tempdir().unwrap();
        let backup_path = create_synthetic_backup_with_chat_storage(root.path());

        let preview = read_iphone_backup_attachment_preview_from_path(
            &backup_path,
            "Message/Media/missing.jpg",
            "missing.jpg",
            AttachmentKind::Photo,
        )
        .unwrap();

        assert!(preview.is_none());
    }

    #[test]
    fn exports_iphone_backup_chat_to_self_contained_html_file() {
        let root = tempdir().unwrap();
        let backup_path = create_synthetic_backup_with_media_chat(root.path());
        let output_path = root.path().join("backup-chat.html");

        let result =
            export_iphone_backup_chat_html_file(&backup_path, "1", &output_path, "Backup Chat")
                .unwrap();
        let html = fs::read_to_string(&output_path).unwrap();

        assert_eq!(result.embedded_attachment_count, 1);
        assert_eq!(result.skipped_attachment_count, 0);
        assert!(!format!("{result:?}").contains(output_path.to_str().unwrap()));
        assert!(html.contains("<title>Backup Chat</title>"));
        assert!(html.contains("data:image/jpeg;base64,ZmFrZSBiYWNrdXAgaW1hZ2U="));
        assert!(html.contains("photo attached"));
    }

    #[test]
    fn exports_whatsapp_zip_to_self_contained_html_file() {
        let root = tempdir().unwrap();
        let source_path = root.path().join("chat.zip");
        let output_path = root.path().join("chat.html");
        create_synthetic_export_zip(&source_path);

        let result =
            export_whatsapp_export_html_file(&source_path, &output_path, "Exported chat").unwrap();
        let html = fs::read_to_string(&output_path).unwrap();

        assert_eq!(result.embedded_attachment_count, 1);
        assert_eq!(result.skipped_attachment_count, 0);
        assert!(html.contains("<title>Exported chat</title>"));
        assert!(html.contains("data:image/jpeg;base64,ZmFrZSBpbWFnZQ=="));
        assert!(html.contains("hello"));
    }

    fn create_synthetic_export_zip(path: &Path) {
        let file = fs::File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

        writer.start_file("_chat.txt", options).unwrap();
        writer
            .write_all(
                concat!(
                    "[06/23/2026, 09:15:00] Ana: hello\n",
                    "[06/23/2026, 09:16:00] You: 00000001-PHOTO-2026-06-23-09-16-00.jpg\n"
                )
                .as_bytes(),
            )
            .unwrap();
        writer
            .start_file("00000001-PHOTO-2026-06-23-09-16-00.jpg", options)
            .unwrap();
        writer.write_all(b"fake image").unwrap();
        writer.finish().unwrap();
    }

    fn create_synthetic_backup_with_chat_storage(root: &Path) -> std::path::PathBuf {
        let backup_path = root.join("synthetic-device-backup");
        let chat_storage_file_id = "synthetic-chat-storage-file-id";
        fs::create_dir_all(backup_path.join("sy")).unwrap();
        create_manifest_db(&backup_path.join("Manifest.db"), chat_storage_file_id);
        create_chat_storage(&backup_path.join("sy").join(chat_storage_file_id));
        backup_path
    }

    fn create_synthetic_backup_with_media(root: &Path) -> std::path::PathBuf {
        let backup_path = create_synthetic_backup_with_chat_storage(root);
        let manifest_path = backup_path.join("Manifest.db");
        let media_file_id = "media-file-id";
        let media_path = backup_path.join("me").join(media_file_id);

        fs::create_dir_all(backup_path.join("me")).unwrap();
        fs::write(&media_path, b"fake backup image").unwrap();

        let connection = rusqlite::Connection::open(&manifest_path).unwrap();
        connection
            .execute(
                "INSERT INTO Files (fileID, domain, relativePath, flags, file) VALUES (?1, ?2, ?3, ?4, NULL)",
                (
                    media_file_id,
                    "AppDomainGroup-group.net.whatsapp.WhatsApp.shared",
                    "Message/Media/photo.jpg",
                    1_i64,
                ),
            )
            .unwrap();

        backup_path
    }

    fn create_synthetic_backup_with_media_chat(root: &Path) -> std::path::PathBuf {
        let backup_path = root.join("synthetic-device-backup-with-media");
        let chat_storage_file_id = "synthetic-chat-storage-with-media-file-id";
        let media_file_id = "media-file-id";

        fs::create_dir_all(backup_path.join("sy")).unwrap();
        fs::create_dir_all(backup_path.join("me")).unwrap();
        create_manifest_db(&backup_path.join("Manifest.db"), chat_storage_file_id);
        create_chat_storage_with_media(&backup_path.join("sy").join(chat_storage_file_id));
        fs::write(
            backup_path.join("me").join(media_file_id),
            b"fake backup image",
        )
        .unwrap();

        let connection = rusqlite::Connection::open(backup_path.join("Manifest.db")).unwrap();
        connection
            .execute(
                "INSERT INTO Files (fileID, domain, relativePath, flags, file) VALUES (?1, ?2, ?3, ?4, NULL)",
                (
                    media_file_id,
                    "AppDomainGroup-group.net.whatsapp.WhatsApp.shared",
                    "Message/Media/photo.jpg",
                    1_i64,
                ),
            )
            .unwrap();

        backup_path
    }

    fn create_manifest_db(path: &Path, chat_storage_file_id: &str) {
        let connection = rusqlite::Connection::open(path).unwrap();
        connection
            .execute_batch(
                r#"
                CREATE TABLE Files (
                    fileID TEXT PRIMARY KEY,
                    domain TEXT,
                    relativePath TEXT,
                    flags INTEGER,
                    file BLOB
                );
                "#,
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO Files (fileID, domain, relativePath, flags, file) VALUES (?1, ?2, ?3, ?4, NULL)",
                (
                    chat_storage_file_id,
                    "AppDomainGroup-group.net.whatsapp.WhatsApp.shared",
                    "ChatStorage.sqlite",
                    1_i64,
                ),
            )
            .unwrap();
    }

    fn create_chat_storage(path: &Path) {
        let connection = rusqlite::Connection::open(path).unwrap();
        connection
            .execute_batch(
                r#"
                CREATE TABLE ZWACHATSESSION (
                    Z_PK INTEGER PRIMARY KEY,
                    ZCONTACTJID TEXT,
                    ZPARTNERNAME TEXT,
                    ZMESSAGECOUNTER INTEGER,
                    ZLASTMESSAGEDATE REAL
                );
                CREATE TABLE ZWAMESSAGE (
                    Z_PK INTEGER PRIMARY KEY,
                    ZCHATSESSION INTEGER,
                    ZSORT INTEGER,
                    ZISFROMME INTEGER,
                    ZMESSAGEDATE REAL,
                    ZTEXT TEXT,
                    ZFROMJID TEXT
                );

                INSERT INTO ZWACHATSESSION
                    (Z_PK, ZCONTACTJID, ZPARTNERNAME, ZMESSAGECOUNTER, ZLASTMESSAGEDATE)
                VALUES
                    (1, 'backup-chat@s.whatsapp.net', 'Backup Chat', 2, 120);

                INSERT INTO ZWAMESSAGE
                    (Z_PK, ZCHATSESSION, ZSORT, ZISFROMME, ZMESSAGEDATE, ZTEXT, ZFROMJID)
                VALUES
                    (1, 1, 1, 0, 60, 'hello from backup', 'friend@s.whatsapp.net'),
                    (2, 1, 2, 1, 120, 'reply from me', NULL);
                "#,
            )
            .unwrap();
    }

    fn create_chat_storage_with_media(path: &Path) {
        let connection = rusqlite::Connection::open(path).unwrap();
        connection
            .execute_batch(
                r#"
                CREATE TABLE ZWACHATSESSION (
                    Z_PK INTEGER PRIMARY KEY,
                    ZCONTACTJID TEXT,
                    ZPARTNERNAME TEXT,
                    ZMESSAGECOUNTER INTEGER,
                    ZLASTMESSAGEDATE REAL
                );
                CREATE TABLE ZWAMESSAGE (
                    Z_PK INTEGER PRIMARY KEY,
                    ZCHATSESSION INTEGER,
                    ZSORT INTEGER,
                    ZISFROMME INTEGER,
                    ZMESSAGEDATE REAL,
                    ZTEXT TEXT,
                    ZFROMJID TEXT,
                    ZMEDIAITEM INTEGER
                );
                CREATE TABLE ZWAMEDIAITEM (
                    Z_PK INTEGER PRIMARY KEY,
                    ZMESSAGE INTEGER,
                    ZMEDIALOCALPATH TEXT,
                    ZVCARDSTRING TEXT,
                    ZTITLE TEXT,
                    ZFILESIZE INTEGER
                );

                INSERT INTO ZWACHATSESSION
                    (Z_PK, ZCONTACTJID, ZPARTNERNAME, ZMESSAGECOUNTER, ZLASTMESSAGEDATE)
                VALUES
                    (1, 'backup-chat@s.whatsapp.net', 'Backup Chat', 2, 120);

                INSERT INTO ZWAMESSAGE
                    (Z_PK, ZCHATSESSION, ZSORT, ZISFROMME, ZMESSAGEDATE, ZTEXT, ZFROMJID, ZMEDIAITEM)
                VALUES
                    (1, 1, 1, 0, 60, 'hello from backup', 'friend@s.whatsapp.net', NULL),
                    (2, 1, 2, 1, 120, 'photo attached', NULL, 10);

                INSERT INTO ZWAMEDIAITEM
                    (Z_PK, ZMESSAGE, ZMEDIALOCALPATH, ZVCARDSTRING, ZTITLE, ZFILESIZE)
                VALUES
                    (10, 2, 'Message/Media/photo.jpg', 'image/jpeg', 'photo.jpg', 17);
                "#,
            )
            .unwrap();
    }
}
