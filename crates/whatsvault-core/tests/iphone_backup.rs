use std::{fs, path::Path};

use rusqlite::Connection;
use tempfile::tempdir;
use whatsvault_core::sources::iphone_backup::{
    discover_backup_candidates, find_whatsapp_manifest_file_by_relative_path,
    find_whatsapp_manifest_files, physical_backup_file_path, read_backup_metadata,
    read_manifest_files, resolve_whatsapp_media_file_path,
};

fn create_manifest_db(path: &Path) {
    let connection = Connection::open(path).unwrap();
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
            CREATE INDEX FilesDomainIdx ON Files(domain);
            CREATE INDEX FilesRelativePathIdx ON Files(relativePath);
            CREATE INDEX FilesFlagsIdx ON Files(flags);
            "#,
        )
        .unwrap();
}

fn insert_manifest_file(connection: &Connection, file_id: &str, domain: &str, relative_path: &str) {
    connection
        .execute(
            "INSERT INTO Files (fileID, domain, relativePath, flags, file) VALUES (?1, ?2, ?3, ?4, NULL)",
            (file_id, domain, relative_path, 1_i64),
        )
        .unwrap();
}

#[test]
fn missing_backup_root_returns_empty_candidates() {
    let root = tempdir().unwrap().path().join("missing");

    let candidates = discover_backup_candidates(root).unwrap();

    assert!(candidates.is_empty());
}

#[test]
fn discovers_backup_candidates_that_have_manifest_db() {
    let root = tempdir().unwrap();
    let backup = root.path().join("device-backup-a");
    let ignored = root.path().join("not-a-backup");
    fs::create_dir_all(&backup).unwrap();
    fs::create_dir_all(&ignored).unwrap();
    fs::write(backup.join("Manifest.db"), b"").unwrap();
    fs::write(backup.join("Manifest.plist"), b"synthetic").unwrap();
    fs::write(backup.join("Info.plist"), b"synthetic").unwrap();
    fs::write(backup.join("Status.plist"), b"synthetic").unwrap();

    let candidates = discover_backup_candidates(root.path()).unwrap();

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].id, "device-backup-a");
    assert!(candidates[0].manifest_db_path.ends_with("Manifest.db"));
    assert!(candidates[0].manifest_plist_path.is_some());
    assert!(candidates[0].info_plist_path.is_some());
    assert!(candidates[0].status_plist_path.is_some());
}

#[test]
fn reads_backup_metadata_from_synthetic_plists() {
    let root = tempdir().unwrap();
    let backup = root.path().join("device-backup-a");
    fs::create_dir_all(&backup).unwrap();
    fs::write(backup.join("Manifest.db"), b"").unwrap();
    fs::write(
        backup.join("Info.plist"),
        br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Device Name</key>
  <string>Example iPhone</string>
  <key>Product Name</key>
  <string>iPhone 15 Pro</string>
  <key>Product Type</key>
  <string>iPhone16,1</string>
  <key>Product Version</key>
  <string>18.5</string>
</dict>
</plist>
"#,
    )
    .unwrap();
    fs::write(
        backup.join("Status.plist"),
        br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Date</key>
  <date>2026-06-23T10:00:00Z</date>
</dict>
</plist>
"#,
    )
    .unwrap();
    fs::write(
        backup.join("Manifest.plist"),
        br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>IsEncrypted</key>
  <false/>
</dict>
</plist>
"#,
    )
    .unwrap();

    let candidate = discover_backup_candidates(root.path()).unwrap().remove(0);
    let metadata = read_backup_metadata(&candidate).unwrap();

    assert_eq!(metadata.device_name.as_deref(), Some("Example iPhone"));
    assert_eq!(metadata.product_name.as_deref(), Some("iPhone 15 Pro"));
    assert_eq!(metadata.product_type.as_deref(), Some("iPhone16,1"));
    assert_eq!(metadata.product_version.as_deref(), Some("18.5"));
    assert_eq!(
        metadata.last_backup_date.as_deref(),
        Some("2026-06-23T10:00:00Z")
    );
    assert_eq!(metadata.is_encrypted, Some(false));
}

#[test]
fn reads_manifest_files_from_synthetic_manifest_db() {
    let root = tempdir().unwrap();
    let manifest = root.path().join("Manifest.db");
    create_manifest_db(&manifest);
    let connection = Connection::open(&manifest).unwrap();
    insert_manifest_file(
        &connection,
        "abc123",
        "AppDomainGroup-group.net.whatsapp.WhatsApp.shared",
        "ChatStorage.sqlite",
    );
    insert_manifest_file(
        &connection,
        "def456",
        "AppDomainGroup-group.net.whatsapp.WhatsApp.shared",
        "Message/Media/image.jpg",
    );

    let files = read_manifest_files(&manifest).unwrap();

    assert_eq!(files.len(), 2);
    assert_eq!(files[0].file_id, "abc123");
    assert_eq!(files[0].relative_path, "ChatStorage.sqlite");
    assert_eq!(files[1].relative_path, "Message/Media/image.jpg");
}

#[test]
fn finds_whatsapp_targets_from_manifest_db() {
    let root = tempdir().unwrap();
    let manifest = root.path().join("Manifest.db");
    create_manifest_db(&manifest);
    let connection = Connection::open(&manifest).unwrap();
    insert_manifest_file(
        &connection,
        "chat-storage-file-id",
        "AppDomainGroup-group.net.whatsapp.WhatsApp.shared",
        "ChatStorage.sqlite",
    );
    insert_manifest_file(
        &connection,
        "contacts-file-id",
        "AppDomainGroup-group.net.whatsapp.WhatsApp.shared",
        "ContactsV2.sqlite",
    );
    insert_manifest_file(
        &connection,
        "media-file-id",
        "AppDomainGroup-group.net.whatsapp.WhatsApp.shared",
        "Message/Media/photo.jpg",
    );
    insert_manifest_file(
        &connection,
        "other-file-id",
        "AppDomainGroup-group.example.OtherApp",
        "ChatStorage.sqlite",
    );

    let whatsapp = find_whatsapp_manifest_files(&manifest).unwrap();

    assert_eq!(
        whatsapp
            .chat_storage
            .as_ref()
            .map(|file| file.file_id.as_str()),
        Some("chat-storage-file-id")
    );
    assert_eq!(
        whatsapp.contacts.as_ref().map(|file| file.file_id.as_str()),
        Some("contacts-file-id")
    );
    assert_eq!(whatsapp.media.len(), 1);
    assert_eq!(whatsapp.media[0].file_id, "media-file-id");
}

#[test]
fn resolves_manifest_file_ids_to_modern_backup_storage_paths() {
    let root = tempdir().unwrap();
    let backup_root = root.path().join("device-backup-a");

    let resolved = physical_backup_file_path(&backup_root, "synthetic-chat-storage-file-id");

    assert_eq!(
        resolved,
        backup_root
            .join("sy")
            .join("synthetic-chat-storage-file-id")
    );
}

#[test]
fn resolves_whatsapp_media_relative_paths_to_physical_backup_files() {
    let root = tempdir().unwrap();
    let backup_root = root.path().join("device-backup-a");
    let manifest = backup_root.join("Manifest.db");
    fs::create_dir_all(&backup_root).unwrap();
    create_manifest_db(&manifest);
    let connection = Connection::open(&manifest).unwrap();
    insert_manifest_file(
        &connection,
        "media-file-id",
        "AppDomainGroup-group.net.whatsapp.WhatsApp.shared",
        "Message/Media/photo.jpg",
    );

    let resolved =
        resolve_whatsapp_media_file_path(&backup_root, &manifest, "\\Message\\Media\\photo.jpg")
            .unwrap();

    assert_eq!(resolved, Some(backup_root.join("me").join("media-file-id")));
}

#[test]
fn finds_whatsapp_manifest_file_by_normalized_relative_path() {
    let root = tempdir().unwrap();
    let manifest = root.path().join("Manifest.db");
    create_manifest_db(&manifest);
    let connection = Connection::open(&manifest).unwrap();
    insert_manifest_file(
        &connection,
        "media-file-id",
        "AppDomainGroup-group.net.whatsapp.WhatsApp.shared",
        "Message/Media/photo.jpg",
    );

    let file = find_whatsapp_manifest_file_by_relative_path(&manifest, "/Message/Media/photo.jpg")
        .unwrap()
        .unwrap();

    assert_eq!(file.file_id, "media-file-id");
    assert_eq!(file.relative_path, "Message/Media/photo.jpg");
}
