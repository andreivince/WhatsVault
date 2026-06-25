use std::path::{Path, PathBuf};

use whatsvault_core::sources::iphone_backup::{
    default_backup_roots, discover_backup_candidates, find_whatsapp_manifest_files,
    physical_backup_file_path, IphoneBackupError,
};
use whatsvault_core::{
    whatsapp::chat_storage::{summarize_chat_storage, ChatStorageError},
    ChatStorageSummary,
};

#[derive(Debug, thiserror::Error)]
pub enum ProofError {
    #[error(transparent)]
    IphoneBackup(#[from] IphoneBackupError),
    #[error(transparent)]
    ChatStorage(#[from] ChatStorageError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofReport {
    pub roots_checked: usize,
    pub backups: Vec<BackupProof>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupProof {
    pub backup_index: usize,
    pub has_info_plist: bool,
    pub has_status_plist: bool,
    pub has_chat_storage: bool,
    pub chat_storage_file_exists: bool,
    pub chat_storage_summary: Option<ChatStorageCountsProof>,
    pub has_contacts: bool,
    pub media_file_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatStorageCountsProof {
    pub message_count: Option<u64>,
    pub chat_count: Option<u64>,
    pub media_item_count: Option<u64>,
}

impl From<ChatStorageSummary> for ChatStorageCountsProof {
    fn from(summary: ChatStorageSummary) -> Self {
        Self {
            message_count: summary.message_count,
            chat_count: summary.chat_count,
            media_item_count: summary.media_item_count,
        }
    }
}

pub fn build_report(roots: Vec<PathBuf>) -> Result<ProofReport, ProofError> {
    let roots = if roots.is_empty() {
        default_backup_roots()
    } else {
        roots
    };

    let mut backups = Vec::new();

    for root in &roots {
        let candidates = discover_backup_candidates(root)?;

        for candidate in candidates {
            let whatsapp = find_whatsapp_manifest_files(&candidate.manifest_db_path)?;
            let chat_storage_path = whatsapp
                .chat_storage
                .as_ref()
                .map(|file| physical_backup_file_path(Path::new(&candidate.path), &file.file_id));
            let chat_storage_file_exists = chat_storage_path
                .as_ref()
                .map(|path| path.is_file())
                .unwrap_or(false);
            let chat_storage_summary = match chat_storage_path.as_ref() {
                Some(path) if path.is_file() => Some(summarize_chat_storage(path)?.into()),
                _ => None,
            };

            backups.push(BackupProof {
                backup_index: backups.len() + 1,
                has_info_plist: candidate.info_plist_path.is_some(),
                has_status_plist: candidate.status_plist_path.is_some(),
                has_chat_storage: whatsapp.chat_storage.is_some(),
                chat_storage_file_exists,
                chat_storage_summary,
                has_contacts: whatsapp.contacts.is_some(),
                media_file_count: whatsapp.media.len(),
            });
        }
    }

    Ok(ProofReport {
        roots_checked: roots.len(),
        backups,
    })
}

pub fn render_report(report: &ProofReport) -> String {
    let mut output = String::new();
    output.push_str("WhatsVault backup proof\n");
    output.push_str(&format!("backup_roots_checked: {}\n", report.roots_checked));
    output.push_str(&format!("backup_candidates: {}\n", report.backups.len()));

    if report.backups.is_empty() {
        output.push_str("status: no local iPhone backups found\n");
        return output;
    }

    for backup in &report.backups {
        output.push_str(&format!("\nbackup #{}\n", backup.backup_index));
        output.push_str(&format!("info_plist: {}\n", yes_no(backup.has_info_plist)));
        output.push_str(&format!(
            "status_plist: {}\n",
            yes_no(backup.has_status_plist)
        ));
        output.push_str(&format!(
            "whatsapp_chat_storage_manifest_entry: {}\n",
            yes_no(backup.has_chat_storage)
        ));
        output.push_str(&format!(
            "whatsapp_chat_storage_file_exists: {}\n",
            yes_no(backup.chat_storage_file_exists)
        ));
        output.push_str(&format!(
            "whatsapp_message_count: {}\n",
            count_or_unknown(
                backup
                    .chat_storage_summary
                    .as_ref()
                    .and_then(|summary| summary.message_count)
            )
        ));
        output.push_str(&format!(
            "whatsapp_chat_count: {}\n",
            count_or_unknown(
                backup
                    .chat_storage_summary
                    .as_ref()
                    .and_then(|summary| summary.chat_count)
            )
        ));
        output.push_str(&format!(
            "whatsapp_media_item_count: {}\n",
            count_or_unknown(
                backup
                    .chat_storage_summary
                    .as_ref()
                    .and_then(|summary| summary.media_item_count)
            )
        ));
        output.push_str(&format!(
            "whatsapp_contacts_manifest_entry: {}\n",
            yes_no(backup.has_contacts)
        ));
        output.push_str(&format!(
            "whatsapp_media_manifest_entries: {}\n",
            backup.media_file_count
        ));
    }

    output
}

fn count_or_unknown(value: Option<u64>) -> String {
    value
        .map(|count| count.to_string())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::{build_report, render_report, BackupProof, ChatStorageCountsProof, ProofReport};

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

    fn insert_manifest_file(
        connection: &Connection,
        file_id: &str,
        domain: &str,
        relative_path: &str,
    ) {
        connection
            .execute(
                "INSERT INTO Files (fileID, domain, relativePath, flags, file) VALUES (?1, ?2, ?3, ?4, NULL)",
                (file_id, domain, relative_path, 1_i64),
            )
            .unwrap();
    }

    fn create_chat_storage(path: &Path) {
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch(
                r#"
                CREATE TABLE ZWAMESSAGE (
                    Z_PK INTEGER PRIMARY KEY,
                    ZTEXT TEXT
                );
                CREATE TABLE ZWACHATSESSION (
                    Z_PK INTEGER PRIMARY KEY,
                    ZCONTACTJID TEXT
                );
                CREATE TABLE ZWAMEDIAITEM (
                    Z_PK INTEGER PRIMARY KEY,
                    ZMESSAGE INTEGER
                );

                INSERT INTO ZWAMESSAGE (ZTEXT) VALUES ('hello'), ('reply'), ('photo');
                INSERT INTO ZWACHATSESSION (ZCONTACTJID) VALUES ('one@s.whatsapp.net'), ('group@g.us');
                INSERT INTO ZWAMEDIAITEM (ZMESSAGE) VALUES (3);
                "#,
            )
            .unwrap();
    }

    #[test]
    fn empty_report_does_not_include_private_paths_or_ids() {
        let report = ProofReport {
            roots_checked: 1,
            backups: Vec::new(),
        };

        assert_eq!(
            render_report(&report),
            concat!(
                "WhatsVault backup proof\n",
                "backup_roots_checked: 1\n",
                "backup_candidates: 0\n",
                "status: no local iPhone backups found\n"
            )
        );
    }

    #[test]
    fn backup_report_uses_counts_and_booleans_only() {
        let report = ProofReport {
            roots_checked: 1,
            backups: vec![BackupProof {
                backup_index: 1,
                has_info_plist: true,
                has_status_plist: false,
                has_chat_storage: true,
                chat_storage_file_exists: true,
                chat_storage_summary: Some(ChatStorageCountsProof {
                    message_count: Some(10),
                    chat_count: Some(2),
                    media_item_count: Some(4),
                }),
                has_contacts: true,
                media_file_count: 42,
            }],
        };

        let rendered = render_report(&report);

        assert!(rendered.contains("backup #1"));
        assert!(rendered.contains("whatsapp_chat_storage_file_exists: yes"));
        assert!(rendered.contains("whatsapp_message_count: 10"));
        assert!(rendered.contains("whatsapp_chat_count: 2"));
        assert!(rendered.contains("whatsapp_media_item_count: 4"));
        assert!(rendered.contains("whatsapp_media_manifest_entries: 42"));
        assert!(!rendered.contains("/Users/"));
        assert!(!rendered.contains("fileID"));
    }

    #[test]
    fn builds_report_from_synthetic_backup_without_private_identifiers() {
        let root = tempdir().unwrap();
        let backup = root.path().join("synthetic-device-backup");
        fs::create_dir_all(backup.join("sy")).unwrap();
        fs::write(backup.join("Info.plist"), b"synthetic").unwrap();
        fs::write(backup.join("Status.plist"), b"synthetic").unwrap();
        create_chat_storage(&backup.join("sy").join("synthetic-chat-storage-file-id"));

        let manifest = backup.join("Manifest.db");
        create_manifest_db(&manifest);
        let connection = Connection::open(&manifest).unwrap();
        insert_manifest_file(
            &connection,
            "synthetic-chat-storage-file-id",
            "AppDomainGroup-group.net.whatsapp.WhatsApp.shared",
            "ChatStorage.sqlite",
        );
        insert_manifest_file(
            &connection,
            "synthetic-photo-media-file-id",
            "AppDomainGroup-group.net.whatsapp.WhatsApp.shared",
            "Message/Media/photo.jpg",
        );

        let report = build_report(vec![root.path().to_path_buf()]).unwrap();
        let rendered = render_report(&report);

        assert_eq!(report.roots_checked, 1);
        assert_eq!(report.backups.len(), 1);
        assert!(rendered.contains("backup_candidates: 1"));
        assert!(rendered.contains("whatsapp_chat_storage_file_exists: yes"));
        assert!(rendered.contains("whatsapp_message_count: 3"));
        assert!(rendered.contains("whatsapp_chat_count: 2"));
        assert!(rendered.contains("whatsapp_media_item_count: 1"));
        assert!(rendered.contains("whatsapp_media_manifest_entries: 1"));
        assert!(!rendered.contains("synthetic-device-backup"));
        assert!(!rendered.contains("synthetic-chat-storage-file-id"));
        assert!(!rendered.contains("synthetic-photo-media-file-id"));
    }
}
