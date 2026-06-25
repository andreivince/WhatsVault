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
mod tests {
    use std::{fs, io::Write, path::Path};

    use tempfile::tempdir;
    use whatsvault_core::{media::attachment_media_type, AttachmentKind, ImportIssueCode};
    use zip::write::SimpleFileOptions;

    use super::{
        backup_candidate_dto, backup_display_name, backup_product_label,
        export_iphone_backup_chat_html_file, export_whatsapp_export_html_file,
        import_iphone_backup_chat_from_path, list_iphone_backup_chats_from_path,
        read_iphone_backup_attachment_preview_from_path, register_backup_candidate_dtos,
        safe_html_default_filename, search_iphone_backup_chat_from_path,
        search_iphone_backup_chats_from_path, source_display_name, PublicError, SourceRegistry,
        BACKUP_CHAT_IMPORT_MAX_MESSAGES, BACKUP_CHAT_LIST_MAX_ROWS, BACKUP_CHAT_SEARCH_MAX_RESULTS,
        BACKUP_CHAT_SEARCH_MAX_ROWS, DEFAULT_WHATSAPP_EXPORT_IMPORT_MAX_MESSAGES,
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
    fn selected_backup_candidates_reuse_opaque_backup_handles() {
        let root = tempdir().unwrap();
        let backup_path = root.path().join("selected-device-backup");
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
            id: "selected-device-backup".to_owned(),
            path: backup_path.to_string_lossy().into_owned(),
            manifest_db_path: manifest_db_path.to_string_lossy().into_owned(),
            manifest_plist_path: None,
            info_plist_path: None,
            status_plist_path: None,
        };
        let registry = std::sync::Mutex::new(SourceRegistry::default());

        let dtos = register_backup_candidate_dtos(&registry, &[candidate]).unwrap();

        assert_eq!(dtos.len(), 1);
        assert_eq!(dtos[0].handle, "backup-source-1");
        assert!(!dtos[0].handle.contains("selected-device-backup"));
        assert_eq!(
            registry
                .lock()
                .unwrap()
                .backup_path("backup-source-1")
                .as_deref(),
            Some(backup_path.as_path())
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

        let result = list_iphone_backup_chats_from_path(&backup_path).unwrap();
        let chats = result.chats;

        assert_eq!(chats.len(), 1);
        assert!(!result.is_truncated);
        assert_eq!(result.limit, BACKUP_CHAT_LIST_MAX_ROWS);
        assert_eq!(chats[0].id, "1");
        assert_eq!(chats[0].title, "Backup Chat");
        assert_eq!(chats[0].message_count, 2);
    }

    #[test]
    fn lists_iphone_backup_chats_with_visible_bound_for_large_backups() {
        let root = tempdir().unwrap();
        let backup_path =
            create_synthetic_backup_with_many_chats(root.path(), BACKUP_CHAT_LIST_MAX_ROWS + 7);

        let result = list_iphone_backup_chats_from_path(&backup_path).unwrap();

        assert_eq!(result.chats.len(), BACKUP_CHAT_LIST_MAX_ROWS);
        assert!(result.is_truncated);
        assert_eq!(result.limit, BACKUP_CHAT_LIST_MAX_ROWS);
    }

    #[test]
    fn searches_iphone_backup_chats_outside_visible_chat_list_window() {
        let root = tempdir().unwrap();
        let backup_path =
            create_synthetic_backup_with_search_target(root.path(), BACKUP_CHAT_LIST_MAX_ROWS + 7);

        let listed = list_iphone_backup_chats_from_path(&backup_path).unwrap();
        let searched = search_iphone_backup_chats_from_path(&backup_path, "needle").unwrap();

        assert!(listed.is_truncated);
        assert!(!listed
            .chats
            .iter()
            .any(|chat| chat.title == "Needle Archive"));
        assert_eq!(searched.chats.len(), 1);
        assert_eq!(searched.chats[0].title, "Needle Archive");
        assert!(!searched.is_truncated);
        assert_eq!(searched.limit, BACKUP_CHAT_SEARCH_MAX_ROWS);
    }

    #[test]
    fn searches_iphone_backup_chats_with_visible_bound_for_broad_queries() {
        let root = tempdir().unwrap();
        let backup_path = create_synthetic_backup_with_search_target(
            root.path(),
            BACKUP_CHAT_SEARCH_MAX_ROWS + 7,
        );

        let result = search_iphone_backup_chats_from_path(&backup_path, "Recent Chat").unwrap();

        assert_eq!(result.chats.len(), BACKUP_CHAT_SEARCH_MAX_ROWS);
        assert!(result.is_truncated);
        assert_eq!(result.limit, BACKUP_CHAT_SEARCH_MAX_ROWS);
    }

    #[test]
    fn serializes_iphone_backup_chat_summaries_for_react_without_snake_case_drift() {
        let root = tempdir().unwrap();
        let backup_path = create_synthetic_backup_with_chat_storage(root.path());

        let result = list_iphone_backup_chats_from_path(&backup_path).unwrap();
        let serialized = serde_json::to_string(&result).unwrap();

        assert!(serialized.contains("chats"));
        assert!(serialized.contains("isTruncated"));
        assert!(serialized.contains("latestMessage"));
        assert!(serialized.contains("latestMessageTimestamp"));
        assert!(serialized.contains("messageCount"));
        assert!(serialized.contains("attachmentCount"));
        assert!(!serialized.contains("latest_message"));
        assert!(!serialized.contains("message_count"));
    }

    #[test]
    fn serializes_iphone_backup_chat_search_for_react_without_snake_case_drift() {
        let root = tempdir().unwrap();
        let backup_path = create_synthetic_backup_with_search_target(
            root.path(),
            BACKUP_CHAT_SEARCH_MAX_ROWS + 7,
        );

        let result = search_iphone_backup_chats_from_path(&backup_path, "Recent Chat").unwrap();
        let serialized = serde_json::to_string(&result).unwrap();

        assert!(serialized.contains("chats"));
        assert!(serialized.contains("isTruncated"));
        assert!(serialized.contains("latestMessage"));
        assert!(serialized.contains("messageCount"));
        assert!(serialized.contains("attachmentCount"));
        assert!(!serialized.contains("is_truncated"));
        assert!(!serialized.contains("latest_message"));
        assert!(!serialized.contains("message_count"));
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
    fn imports_bounded_recent_iphone_backup_chat_window_for_large_threads() {
        let root = tempdir().unwrap();
        let message_count = BACKUP_CHAT_IMPORT_MAX_MESSAGES + 3;
        let backup_path = create_synthetic_backup_with_large_chat(root.path(), message_count);

        let imported = import_iphone_backup_chat_from_path(&backup_path, "1").unwrap();

        assert_eq!(imported.messages.len(), BACKUP_CHAT_IMPORT_MAX_MESSAGES);
        assert_eq!(imported.issues.len(), 1);
        assert_eq!(
            imported.issues[0].code,
            ImportIssueCode::MessageWindowTruncated
        );
        assert!(imported.issues[0].message.contains("latest 2000 messages"));
        let first_expected = format!(
            "message {}",
            message_count - BACKUP_CHAT_IMPORT_MAX_MESSAGES + 1
        );
        let last_expected = format!("message {message_count}");
        assert_eq!(imported.messages[0].body, first_expected);
        assert_eq!(
            imported
                .messages
                .last()
                .map(|message| message.body.as_str()),
            Some(last_expected.as_str())
        );
    }

    #[test]
    fn searches_iphone_backup_chat_from_resolved_chat_storage() {
        let root = tempdir().unwrap();
        let message_count = BACKUP_CHAT_SEARCH_MAX_RESULTS + 3;
        let backup_path = create_synthetic_backup_with_large_chat(root.path(), message_count);

        let result = search_iphone_backup_chat_from_path(&backup_path, "1", "message").unwrap();

        assert_eq!(
            result.imported.messages.len(),
            BACKUP_CHAT_SEARCH_MAX_RESULTS
        );
        assert!(result.is_truncated);
        assert_eq!(result.limit, BACKUP_CHAT_SEARCH_MAX_RESULTS);
        assert_eq!(
            result.imported.issues[0].code,
            ImportIssueCode::SearchResultsTruncated
        );
        assert_eq!(result.imported.messages[0].body, "message 4");
        let last_expected = format!("message {message_count}");
        assert_eq!(
            result
                .imported
                .messages
                .last()
                .map(|message| message.body.as_str()),
            Some(last_expected.as_str())
        );
    }

    #[test]
    fn serializes_iphone_backup_search_results_for_react_without_snake_case_drift() {
        let root = tempdir().unwrap();
        let backup_path = create_synthetic_backup_with_chat_storage(root.path());

        let result = search_iphone_backup_chat_from_path(&backup_path, "1", "backup").unwrap();
        let serialized = serde_json::to_string(&result).unwrap();

        assert!(serialized.contains("imported"));
        assert!(serialized.contains("isTruncated"));
        assert!(serialized.contains("limit"));
        assert!(!serialized.contains("is_truncated"));
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
        assert_eq!(result.exported_message_count, 2);
        assert_eq!(result.skipped_message_count, 0);
        assert!(!format!("{result:?}").contains(output_path.to_str().unwrap()));
        assert!(html.contains("<title>Backup Chat</title>"));
        assert!(html.contains("data:image/jpeg;base64,ZmFrZSBiYWNrdXAgaW1hZ2U="));
        assert!(html.contains("photo attached"));
    }

    #[test]
    fn exports_iphone_backup_chat_html_with_bounded_recent_messages() {
        let root = tempdir().unwrap();
        let message_count = BACKUP_CHAT_IMPORT_MAX_MESSAGES + 3;
        let backup_path = create_synthetic_backup_with_large_chat(root.path(), message_count);
        let output_path = root.path().join("large-backup-chat.html");

        let result =
            export_iphone_backup_chat_html_file(&backup_path, "1", &output_path, "Large Chat")
                .unwrap();
        let html = fs::read_to_string(&output_path).unwrap();

        assert_eq!(
            result.exported_message_count,
            BACKUP_CHAT_IMPORT_MAX_MESSAGES
        );
        assert_eq!(result.skipped_message_count, 3);
        assert!(!html.contains("<p class=\"message-body\">message 1</p>"));
        assert!(html.contains(&format!("message {message_count}")));
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
        assert_eq!(result.exported_message_count, 2);
        assert_eq!(result.skipped_message_count, 0);
        assert!(html.contains("<title>Exported chat</title>"));
        assert!(html.contains("data:image/jpeg;base64,ZmFrZSBpbWFnZQ=="));
        assert!(html.contains("hello"));
    }

    #[test]
    fn exports_whatsapp_zip_html_with_bounded_recent_messages() {
        let root = tempdir().unwrap();
        let source_path = root.path().join("large-chat.zip");
        let output_path = root.path().join("large-chat.html");
        let message_count = DEFAULT_WHATSAPP_EXPORT_IMPORT_MAX_MESSAGES + 3;
        create_synthetic_large_export_zip(&source_path, message_count);

        let result =
            export_whatsapp_export_html_file(&source_path, &output_path, "Large Export").unwrap();
        let html = fs::read_to_string(&output_path).unwrap();

        assert_eq!(
            result.exported_message_count,
            DEFAULT_WHATSAPP_EXPORT_IMPORT_MAX_MESSAGES
        );
        assert_eq!(result.skipped_message_count, 3);
        assert!(!html.contains("<p class=\"message-body\">message 1</p>"));
        assert!(html.contains(&format!("message {message_count}")));
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

    fn create_synthetic_large_export_zip(path: &Path, message_count: usize) {
        let file = fs::File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

        writer.start_file("_chat.txt", options).unwrap();
        for index in 1..=message_count {
            writeln!(
                writer,
                "[06/23/2026, 09:15:{:02}] Ana: message {index}",
                index % 60
            )
            .unwrap();
        }
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

    fn create_synthetic_backup_with_large_chat(
        root: &Path,
        message_count: usize,
    ) -> std::path::PathBuf {
        let chat_storage_file_id = "synthetic-chat-storage-file-id";
        let backup_path = root.join("synthetic-device-backup-with-large-chat");

        fs::create_dir_all(backup_path.join("sy")).unwrap();
        create_manifest_db(&backup_path.join("Manifest.db"), chat_storage_file_id);
        create_large_chat_storage(
            &backup_path.join("sy").join(chat_storage_file_id),
            message_count,
        );

        backup_path
    }

    fn create_synthetic_backup_with_many_chats(
        root: &Path,
        chat_count: usize,
    ) -> std::path::PathBuf {
        let chat_storage_file_id = "synthetic-chat-storage-file-id";
        let backup_path = root.join("synthetic-device-backup-with-many-chats");

        fs::create_dir_all(backup_path.join("sy")).unwrap();
        create_manifest_db(&backup_path.join("Manifest.db"), chat_storage_file_id);
        create_many_chat_storage(
            &backup_path.join("sy").join(chat_storage_file_id),
            chat_count,
        );

        backup_path
    }

    fn create_synthetic_backup_with_search_target(
        root: &Path,
        recent_chat_count: usize,
    ) -> std::path::PathBuf {
        let chat_storage_file_id = "synthetic-chat-storage-file-id";
        let backup_path = root.join("synthetic-device-backup-with-searchable-chats");

        fs::create_dir_all(backup_path.join("sy")).unwrap();
        create_manifest_db(&backup_path.join("Manifest.db"), chat_storage_file_id);
        create_searchable_chat_storage(
            &backup_path.join("sy").join(chat_storage_file_id),
            recent_chat_count,
        );

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

    fn create_large_chat_storage(path: &Path, message_count: usize) {
        let mut connection = rusqlite::Connection::open(path).unwrap();
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
                    (1, 'large-chat@s.whatsapp.net', 'Large Chat', 0, 0);
                "#,
            )
            .unwrap();

        let transaction = connection.transaction().unwrap();
        {
            let mut statement = transaction
                .prepare(
                    r#"
                    INSERT INTO ZWAMESSAGE
                        (Z_PK, ZCHATSESSION, ZSORT, ZISFROMME, ZMESSAGEDATE, ZTEXT, ZFROMJID)
                    VALUES (?1, 1, ?1, 0, ?1, ?2, 'friend@s.whatsapp.net')
                    "#,
                )
                .unwrap();

            for index in 1..=message_count {
                statement
                    .execute((index as i64, format!("message {index}")))
                    .unwrap();
            }
        }
        transaction.commit().unwrap();
    }

    fn create_many_chat_storage(path: &Path, chat_count: usize) {
        let mut connection = rusqlite::Connection::open(path).unwrap();
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
                "#,
            )
            .unwrap();

        let transaction = connection.transaction().unwrap();
        {
            let mut statement = transaction
                .prepare(
                    r#"
                    INSERT INTO ZWACHATSESSION
                        (Z_PK, ZCONTACTJID, ZPARTNERNAME, ZMESSAGECOUNTER, ZLASTMESSAGEDATE)
                    VALUES (?1, ?2, ?3, 1, ?1)
                    "#,
                )
                .unwrap();

            for index in 1..=chat_count {
                statement
                    .execute((
                        index as i64,
                        format!("chat-{index}@s.whatsapp.net"),
                        format!("Chat {index}"),
                    ))
                    .unwrap();
            }
        }
        transaction.commit().unwrap();
    }

    fn create_searchable_chat_storage(path: &Path, recent_chat_count: usize) {
        let mut connection = rusqlite::Connection::open(path).unwrap();
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
                "#,
            )
            .unwrap();

        let transaction = connection.transaction().unwrap();
        {
            let mut statement = transaction
                .prepare(
                    r#"
                    INSERT INTO ZWACHATSESSION
                        (Z_PK, ZCONTACTJID, ZPARTNERNAME, ZMESSAGECOUNTER, ZLASTMESSAGEDATE)
                    VALUES (?1, ?2, ?3, 1, ?4)
                    "#,
                )
                .unwrap();

            for index in 1..=recent_chat_count {
                statement
                    .execute((
                        index as i64,
                        format!("recent-chat-{index}@s.whatsapp.net"),
                        format!("Recent Chat {index}"),
                        index as i64,
                    ))
                    .unwrap();
            }

            let target_index = recent_chat_count + 1;
            statement
                .execute((
                    target_index as i64,
                    "needle-archive@s.whatsapp.net".to_owned(),
                    "Needle Archive".to_owned(),
                    0_i64,
                ))
                .unwrap();
        }
        transaction.commit().unwrap();
    }
}
