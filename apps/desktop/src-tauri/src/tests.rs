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
    let backup_path =
        create_synthetic_backup_with_search_target(root.path(), BACKUP_CHAT_SEARCH_MAX_ROWS + 7);

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
    let backup_path =
        create_synthetic_backup_with_search_target(root.path(), BACKUP_CHAT_SEARCH_MAX_ROWS + 7);

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
        export_iphone_backup_chat_html_file(&backup_path, "1", &output_path, "Large Chat").unwrap();
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
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

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
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

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

fn create_synthetic_backup_with_many_chats(root: &Path, chat_count: usize) -> std::path::PathBuf {
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
