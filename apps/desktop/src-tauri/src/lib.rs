mod dtos;
mod public_error;
mod source_registry;

use std::{
    fs,
    fs::File,
    path::{Path, PathBuf},
    sync::Mutex,
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::{DialogExt, FilePath};
use whatsvault_core::{
    exports::html::{build_chat_html_export, EmbeddedAttachment, HtmlExportOptions},
    media::attachment_media_type,
    sources::iphone_backup::{
        discover_backup_candidates_from_selected_path, discover_default_backup_candidates,
        find_whatsapp_manifest_file_by_relative_path, normalize_whatsapp_media_relative_path,
        physical_backup_file_path, read_backup_metadata, resolve_whatsapp_media_file_path,
        resolve_whatsapp_media_file_paths, summarize_whatsapp_manifest_files,
        CHAT_STORAGE_RELATIVE_PATH,
    },
    sources::whatsapp_export_zip::{
        import_whatsapp_export_zip_with_options, read_whatsapp_export_attachment,
        read_whatsapp_export_attachments, WhatsappExportImportOptions,
        DEFAULT_WHATSAPP_EXPORT_IMPORT_MAX_MESSAGES,
    },
    whatsapp::chat_storage::{
        count_chat_storage_chat_messages, import_chat_storage_chat_recent,
        list_chat_storage_chats_limited, search_chat_storage_chat_recent,
        search_chat_storage_chats_limited,
    },
    AttachmentKind, BackupCandidate, BackupMetadata, ChatImport, ImportIssue, ImportIssueCode,
};

use dtos::{
    AttachmentPreviewDto, ChatDto, HtmlExportResultDto, IphoneBackupCandidateDto,
    IphoneBackupChatSearchResultDto, IphoneBackupChatsResultDto, LoadedChatSourceDto,
    OpenLocalChatSourceResultDto, WhatsappBackupStatusDto,
};
use public_error::PublicError;
use source_registry::{
    registered_backup_path, registered_export_path, SourceRegistry, SourceRegistryState,
};

const ATTACHMENT_PREVIEW_MAX_BYTES: u64 = 8 * 1024 * 1024;
const ATTACHMENT_EXPORT_MAX_BYTES: u64 = 24 * 1024 * 1024;
const TOTAL_EXPORT_EMBEDDED_MEDIA_MAX_BYTES: u64 = 128 * 1024 * 1024;
const BACKUP_CHAT_LIST_MAX_ROWS: usize = 1_000;
const BACKUP_CHAT_SEARCH_MAX_ROWS: usize = 200;
const BACKUP_CHAT_IMPORT_MAX_MESSAGES: usize = 2_000;
const BACKUP_CHAT_SEARCH_MAX_RESULTS: usize = 500;

#[tauri::command]
fn list_iphone_backups(
    registry: State<'_, SourceRegistryState>,
) -> Result<Vec<IphoneBackupCandidateDto>, String> {
    let candidates = discover_default_backup_candidates().map_err(|_| {
        "Could not scan the default iPhone backup folders on this computer.".to_owned()
    })?;
    register_backup_candidate_dtos(&registry, &candidates)
}

#[tauri::command]
async fn choose_iphone_backup_folder(
    app: AppHandle,
    registry: State<'_, SourceRegistryState>,
) -> Result<Option<Vec<IphoneBackupCandidateDto>>, String> {
    let Some(source_path) = select_iphone_backup_folder_path(&app)? else {
        return Ok(None);
    };
    let candidates = discover_backup_candidates_from_selected_path(&source_path)
        .map_err(|_| "Could not read iPhone backups from the selected folder.".to_owned())?;

    register_backup_candidate_dtos(&registry, &candidates).map(Some)
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
    let imported = import_whatsapp_export_zip_with_options(
        file,
        WhatsappExportImportOptions::recent(DEFAULT_WHATSAPP_EXPORT_IMPORT_MAX_MESSAGES),
    )
    .map(|result| result.imported)
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
) -> Result<IphoneBackupChatsResultDto, String> {
    let backup_path = registered_backup_path(&registry, &backup_handle)?;
    list_iphone_backup_chats_from_path(&backup_path)
}

fn list_iphone_backup_chats_from_path(
    backup_path: &Path,
) -> Result<IphoneBackupChatsResultDto, String> {
    let chat_storage_path = resolved_chat_storage_path(backup_path)?;
    let mut chats = list_chat_storage_chats_limited(
        chat_storage_path,
        BACKUP_CHAT_LIST_MAX_ROWS.saturating_add(1),
    )
    .map_err(|error| PublicError::BackupChatListFailed.redact(error))?;
    let is_truncated = chats.len() > BACKUP_CHAT_LIST_MAX_ROWS;
    if is_truncated {
        chats.truncate(BACKUP_CHAT_LIST_MAX_ROWS);
    }

    Ok(IphoneBackupChatsResultDto {
        chats: chats.into_iter().map(ChatDto::from).collect(),
        is_truncated,
        limit: BACKUP_CHAT_LIST_MAX_ROWS,
    })
}

#[tauri::command]
fn search_iphone_backup_chats(
    backup_handle: String,
    query: String,
    registry: State<'_, SourceRegistryState>,
) -> Result<IphoneBackupChatsResultDto, String> {
    let backup_path = registered_backup_path(&registry, &backup_handle)?;
    search_iphone_backup_chats_from_path(&backup_path, &query)
}

fn search_iphone_backup_chats_from_path(
    backup_path: &Path,
    query: &str,
) -> Result<IphoneBackupChatsResultDto, String> {
    let chat_storage_path = resolved_chat_storage_path(backup_path)?;
    let mut chats = search_chat_storage_chats_limited(
        chat_storage_path,
        query,
        BACKUP_CHAT_SEARCH_MAX_ROWS.saturating_add(1),
    )
    .map_err(|error| PublicError::BackupChatListFailed.redact(error))?;
    let is_truncated = chats.len() > BACKUP_CHAT_SEARCH_MAX_ROWS;
    if is_truncated {
        chats.truncate(BACKUP_CHAT_SEARCH_MAX_ROWS);
    }

    Ok(IphoneBackupChatsResultDto {
        chats: chats.into_iter().map(ChatDto::from).collect(),
        is_truncated,
        limit: BACKUP_CHAT_SEARCH_MAX_ROWS,
    })
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
    let total_message_count = count_chat_storage_chat_messages(&chat_storage_path, chat_id)
        .map_err(|error| PublicError::BackupChatImportFailed.redact(error))?;
    let mut imported = import_chat_storage_chat_recent(
        chat_storage_path,
        chat_id,
        BACKUP_CHAT_IMPORT_MAX_MESSAGES,
    )
    .map_err(|error| PublicError::BackupChatImportFailed.redact(error))?;
    annotate_recent_message_window(&mut imported, total_message_count);

    Ok(imported)
}

#[tauri::command]
fn search_iphone_backup_chat(
    backup_handle: String,
    chat_id: String,
    query: String,
    registry: State<'_, SourceRegistryState>,
) -> Result<IphoneBackupChatSearchResultDto, String> {
    let backup_path = registered_backup_path(&registry, &backup_handle)?;
    search_iphone_backup_chat_from_path(&backup_path, &chat_id, &query)
}

fn search_iphone_backup_chat_from_path(
    backup_path: &Path,
    chat_id: &str,
    query: &str,
) -> Result<IphoneBackupChatSearchResultDto, String> {
    let chat_storage_path = resolved_chat_storage_path(backup_path)?;
    let imported = search_chat_storage_chat_recent(
        chat_storage_path,
        chat_id,
        query,
        BACKUP_CHAT_SEARCH_MAX_RESULTS,
    )
    .map_err(|error| PublicError::BackupChatImportFailed.redact(error))?;
    let is_truncated = imported
        .issues
        .iter()
        .any(|issue| issue.code == ImportIssueCode::SearchResultsTruncated);

    Ok(IphoneBackupChatSearchResultDto {
        imported,
        is_truncated,
        limit: BACKUP_CHAT_SEARCH_MAX_RESULTS,
    })
}

fn annotate_recent_message_window(imported: &mut ChatImport, total_message_count: u64) -> usize {
    let skipped_message_count =
        count_skipped_messages(total_message_count, imported.messages.len());
    if skipped_message_count > 0 {
        imported.issues.push(ImportIssue {
            code: ImportIssueCode::MessageWindowTruncated,
            message: format!(
                "Only the latest {} messages were loaded for performance",
                imported.messages.len()
            ),
        });
    }

    skipped_message_count
}

fn count_skipped_messages(total_message_count: u64, loaded_message_count: usize) -> usize {
    usize::try_from(total_message_count)
        .unwrap_or(usize::MAX)
        .saturating_sub(loaded_message_count)
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
    let import_result = import_whatsapp_export_zip_with_options(
        source_file,
        WhatsappExportImportOptions::recent(DEFAULT_WHATSAPP_EXPORT_IMPORT_MAX_MESSAGES),
    )
    .map_err(|error| PublicError::SelectedSourceImportFailed.redact(error))?;
    let imported = import_result.imported;
    let mut embedded_attachments = Vec::new();
    let mut embedded_media_bytes = 0_u64;
    let mut requested_attachments = Vec::new();

    for attachment in &imported.attachments {
        if attachment.size_bytes > ATTACHMENT_EXPORT_MAX_BYTES {
            continue;
        }
        if embedded_media_bytes.saturating_add(attachment.size_bytes)
            > TOTAL_EXPORT_EMBEDDED_MEDIA_MAX_BYTES
        {
            continue;
        }

        embedded_media_bytes = embedded_media_bytes.saturating_add(attachment.size_bytes);
        requested_attachments.push(attachment);
    }

    let payloads = read_whatsapp_export_attachments(
        File::open(source_path)
            .map_err(|error| PublicError::SelectedSourceUnreadable.redact(error))?,
        requested_attachments
            .iter()
            .map(|attachment| attachment.archive_path.as_str()),
        ATTACHMENT_EXPORT_MAX_BYTES,
    )
    .map_err(|error| PublicError::HtmlExportFailed.redact(error))?;

    for attachment in requested_attachments {
        let Some(payload) = payloads.get(&attachment.archive_path) else {
            continue;
        };
        let Some(media_type) = attachment_media_type(payload.kind, &payload.filename) else {
            continue;
        };

        embedded_attachments.push(EmbeddedAttachment {
            attachment_id: attachment.id.clone(),
            media_type: media_type.to_owned(),
            base64_data: STANDARD.encode(&payload.bytes),
        });
    }

    write_chat_html_export(
        &imported,
        output_path,
        title,
        embedded_attachments,
        import_result.skipped_message_count,
    )
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
    let total_message_count = count_chat_storage_chat_messages(&chat_storage_path, chat_id)
        .map_err(|error| PublicError::BackupChatImportFailed.redact(error))?;
    let imported = import_chat_storage_chat_recent(
        &chat_storage_path,
        chat_id,
        BACKUP_CHAT_IMPORT_MAX_MESSAGES,
    )
    .map_err(|error| PublicError::BackupChatImportFailed.redact(error))?;
    let skipped_message_count =
        count_skipped_messages(total_message_count, imported.messages.len());
    let mut embedded_attachments = Vec::new();
    let mut embedded_media_bytes = 0_u64;
    let mut requested_attachments = Vec::new();

    for attachment in &imported.attachments {
        if attachment.size_bytes > ATTACHMENT_EXPORT_MAX_BYTES {
            continue;
        }
        if embedded_media_bytes.saturating_add(attachment.size_bytes)
            > TOTAL_EXPORT_EMBEDDED_MEDIA_MAX_BYTES
        {
            continue;
        }

        embedded_media_bytes = embedded_media_bytes.saturating_add(attachment.size_bytes);
        requested_attachments.push(attachment);
    }

    let manifest_db_path = backup_path.join("Manifest.db");
    let resolved_media_paths = resolve_whatsapp_media_file_paths(
        backup_path,
        &manifest_db_path,
        requested_attachments
            .iter()
            .map(|attachment| attachment.archive_path.as_str()),
    )
    .map_err(|_| "Could not resolve media from the selected iPhone backup.".to_owned())?;

    for attachment in requested_attachments {
        let archive_path_key = normalize_whatsapp_media_relative_path(&attachment.archive_path);
        let Some(media_path) = resolved_media_paths.get(&archive_path_key) else {
            continue;
        };

        let Some((bytes, _size_bytes)) =
            read_attachment_file_bytes(media_path, ATTACHMENT_EXPORT_MAX_BYTES)?
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
    }

    write_chat_html_export(
        &imported,
        output_path,
        title,
        embedded_attachments,
        skipped_message_count,
    )
}

pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(SourceRegistry::default()))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_iphone_backups,
            choose_iphone_backup_folder,
            open_whatsapp_export,
            list_iphone_backup_chats,
            search_iphone_backup_chats,
            import_iphone_backup_chat,
            search_iphone_backup_chat,
            read_export_attachment_preview,
            read_iphone_backup_attachment_preview,
            export_whatsapp_export_html,
            export_iphone_backup_chat_html
        ])
        .run(tauri::generate_context!())
        .expect("failed to run WhatsVault desktop app");
}

fn register_backup_candidate_dtos(
    registry: &SourceRegistryState,
    candidates: &[BackupCandidate],
) -> Result<Vec<IphoneBackupCandidateDto>, String> {
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

fn write_chat_html_export(
    imported: &ChatImport,
    output_path: &Path,
    title: &str,
    embedded_attachments: Vec<EmbeddedAttachment>,
    skipped_message_count: usize,
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
        exported_message_count: imported.messages.len(),
        skipped_message_count,
    })
}

fn select_iphone_backup_folder_path(app: &AppHandle) -> Result<Option<PathBuf>, String> {
    app.dialog()
        .file()
        .set_title("Choose iPhone backup folder")
        .blocking_pick_folder()
        .map(file_path_to_path_buf)
        .transpose()
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

    read_attachment_file_bytes(&media_path, max_size_bytes)
}

fn read_attachment_file_bytes(
    media_path: &Path,
    max_size_bytes: u64,
) -> Result<Option<(Vec<u8>, u64)>, String> {
    let Ok(metadata) = fs::metadata(media_path) else {
        return Ok(None);
    };
    if !metadata.is_file() || metadata.len() > max_size_bytes {
        return Ok(None);
    }

    let Ok(bytes) = fs::read(media_path) else {
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
    let chat_storage =
        find_whatsapp_manifest_file_by_relative_path(&manifest_db_path, CHAT_STORAGE_RELATIVE_PATH)
            .map_err(|_| "Could not inspect the selected iPhone backup manifest.".to_owned())?
            .ok_or_else(|| {
                "WhatsApp ChatStorage.sqlite was not found in this backup.".to_owned()
            })?;
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
    let whatsapp = summarize_whatsapp_manifest_files(&candidate.manifest_db_path)
        .map(|summary| WhatsappBackupStatusDto {
            manifest_readable: true,
            has_chat_storage: summary.has_chat_storage,
            has_contacts: summary.has_contacts,
            media_file_count: summary.media_file_count,
        })
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
mod tests;
