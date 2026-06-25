use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use thiserror::Error;

use crate::{BackupCandidate, BackupMetadata, ManifestFile, WhatsappManifestFiles};

pub const WHATSAPP_SHARED_DOMAIN: &str = "AppDomainGroup-group.net.whatsapp.WhatsApp.shared";
pub const CHAT_STORAGE_RELATIVE_PATH: &str = "ChatStorage.sqlite";
pub const CONTACTS_RELATIVE_PATH: &str = "ContactsV2.sqlite";
const MACOS_BACKUP_SUFFIX: &[&str] = &["Library", "Application Support", "MobileSync", "Backup"];
const WINDOWS_STORE_BACKUP_SUFFIX: &[&str] = &["Apple", "MobileSync", "Backup"];
const WINDOWS_LEGACY_BACKUP_SUFFIX: &[&str] = &["Apple Computer", "MobileSync", "Backup"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackupPlatform {
    Macos,
    Windows,
    Unsupported,
}

#[derive(Debug, Error)]
pub enum IphoneBackupError {
    #[error("I/O failed while reading iPhone backup: {0}")]
    Io(#[from] std::io::Error),
    #[error("Property list failed while reading iPhone backup metadata: {0}")]
    Plist(#[from] plist::Error),
    #[error("SQLite failed while reading Manifest.db: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

pub type Result<T> = std::result::Result<T, IphoneBackupError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WhatsappManifestSummary {
    pub has_chat_storage: bool,
    pub has_contacts: bool,
    pub media_file_count: usize,
}

pub fn default_backup_roots() -> Vec<PathBuf> {
    backup_roots_from_env(
        std::env::var_os("HOME").map(PathBuf::from),
        std::env::var_os("USERPROFILE").map(PathBuf::from),
        std::env::var_os("APPDATA").map(PathBuf::from),
    )
}

pub fn discover_default_backup_candidates() -> Result<Vec<BackupCandidate>> {
    let mut candidates = Vec::new();

    for root in default_backup_roots() {
        candidates.extend(discover_backup_candidates(root)?);
    }

    candidates.sort_by(|left, right| left.id.cmp(&right.id));
    candidates.dedup_by(|left, right| left.path == right.path);
    Ok(candidates)
}

pub fn discover_backup_candidates<P>(_root: P) -> Result<Vec<BackupCandidate>>
where
    P: AsRef<Path>,
{
    let root = _root.as_ref();
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut candidates = Vec::new();

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let manifest_db_path = path.join("Manifest.db");
        if !manifest_db_path.is_file() {
            continue;
        }

        candidates.push(backup_candidate_from_directory(&path));
    }

    candidates.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(candidates)
}

pub fn discover_backup_candidates_from_selected_path<P>(_path: P) -> Result<Vec<BackupCandidate>>
where
    P: AsRef<Path>,
{
    let path = _path.as_ref();
    if path.join("Manifest.db").is_file() {
        return Ok(vec![backup_candidate_from_directory(path)]);
    }

    discover_backup_candidates(path)
}

pub fn read_backup_metadata(candidate: &BackupCandidate) -> Result<BackupMetadata> {
    let mut metadata = BackupMetadata::default();

    if let Some(info_plist_path) = &candidate.info_plist_path {
        let info = plist::Value::from_file(info_plist_path)?;
        metadata.device_name = plist_string(&info, "Device Name");
        metadata.display_name = plist_string(&info, "Display Name");
        metadata.product_name = plist_string(&info, "Product Name");
        metadata.product_type = plist_string(&info, "Product Type");
        metadata.product_version = plist_string(&info, "Product Version");
        metadata.last_backup_date = plist_string(&info, "Last Backup Date")
            .or_else(|| plist_date_string(&info, "Last Backup Date"));
    }

    if let Some(status_plist_path) = &candidate.status_plist_path {
        let status = plist::Value::from_file(status_plist_path)?;
        metadata.last_backup_date = metadata
            .last_backup_date
            .or_else(|| plist_string(&status, "Date"))
            .or_else(|| plist_date_string(&status, "Date"));
    }

    if let Some(manifest_plist_path) = &candidate.manifest_plist_path {
        let manifest = plist::Value::from_file(manifest_plist_path)?;
        metadata.is_encrypted = plist_bool(&manifest, "IsEncrypted");
    }

    Ok(metadata)
}

pub fn physical_backup_file_path<P>(backup_root: P, file_id: &str) -> PathBuf
where
    P: AsRef<Path>,
{
    let mut path = backup_root.as_ref().to_path_buf();

    if file_id.len() >= 2 {
        path.push(&file_id[0..2]);
    }

    path.push(file_id);
    path
}

pub fn normalize_whatsapp_media_relative_path(path: &str) -> String {
    normalize_manifest_relative_path(path)
}

pub fn resolve_whatsapp_media_file_path<P, Q>(
    backup_root: P,
    manifest_db_path: Q,
    media_relative_path: &str,
) -> Result<Option<PathBuf>>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let normalized_relative_path = normalize_whatsapp_media_relative_path(media_relative_path);
    let mut paths =
        resolve_whatsapp_media_file_paths(backup_root, manifest_db_path, [media_relative_path])?;

    Ok(paths.remove(&normalized_relative_path))
}

pub fn resolve_whatsapp_media_file_paths<P, Q, I, S>(
    backup_root: P,
    manifest_db_path: Q,
    media_relative_paths: I,
) -> Result<HashMap<String, PathBuf>>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let backup_root = backup_root.as_ref();
    let connection = open_manifest_read_only(manifest_db_path)?;
    let mut resolved_paths = HashMap::new();

    for media_relative_path in media_relative_paths {
        let normalized_relative_path =
            normalize_whatsapp_media_relative_path(media_relative_path.as_ref());
        if normalized_relative_path.is_empty()
            || resolved_paths.contains_key(&normalized_relative_path)
        {
            continue;
        }

        let media_candidates =
            whatsapp_media_manifest_relative_path_candidates(&normalized_relative_path);
        for media_candidate in media_candidates {
            if let Some(file) = find_whatsapp_manifest_file_by_relative_path_in_connection(
                &connection,
                &media_candidate,
            )? {
                resolved_paths.insert(
                    normalized_relative_path.clone(),
                    physical_backup_file_path(backup_root, &file.file_id),
                );
                break;
            }
        }
    }

    Ok(resolved_paths)
}

pub fn read_manifest_files<P>(_manifest_db_path: P) -> Result<Vec<ManifestFile>>
where
    P: AsRef<Path>,
{
    let connection = open_manifest_read_only(_manifest_db_path)?;
    let mut statement = connection.prepare(
        r#"
        SELECT fileID, domain, relativePath, flags
        FROM Files
        ORDER BY rowid
        "#,
    )?;

    let rows = statement.query_map([], |row| {
        Ok(ManifestFile {
            file_id: row.get(0)?,
            domain: row.get(1)?,
            relative_path: row.get(2)?,
            flags: row.get(3)?,
        })
    })?;

    let mut files = Vec::new();
    for row in rows {
        files.push(row?);
    }

    Ok(files)
}

pub fn find_whatsapp_manifest_file_by_relative_path<P>(
    _manifest_db_path: P,
    relative_path: &str,
) -> Result<Option<ManifestFile>>
where
    P: AsRef<Path>,
{
    let connection = open_manifest_read_only(_manifest_db_path)?;
    let normalized_relative_path = normalize_manifest_relative_path(relative_path);

    find_whatsapp_manifest_file_by_relative_path_in_connection(
        &connection,
        &normalized_relative_path,
    )
}

fn find_whatsapp_manifest_file_by_relative_path_in_connection(
    connection: &Connection,
    normalized_relative_path: &str,
) -> Result<Option<ManifestFile>> {
    Ok(connection
        .query_row(
            r#"
            SELECT fileID, domain, relativePath, flags
            FROM Files
            WHERE domain = ?1
              AND TRIM(REPLACE(COALESCE(relativePath, ''), char(92), '/'), '/') = ?2
            LIMIT 1
            "#,
            [WHATSAPP_SHARED_DOMAIN, normalized_relative_path],
            |row| {
                Ok(ManifestFile {
                    file_id: row.get(0)?,
                    domain: row.get(1)?,
                    relative_path: row.get(2)?,
                    flags: row.get(3)?,
                })
            },
        )
        .optional()?)
}

pub fn summarize_whatsapp_manifest_files<P>(_manifest_db_path: P) -> Result<WhatsappManifestSummary>
where
    P: AsRef<Path>,
{
    let connection = open_manifest_read_only(_manifest_db_path)?;

    connection
        .query_row(
            r#"
            WITH whatsapp_files AS (
                SELECT TRIM(REPLACE(COALESCE(relativePath, ''), char(92), '/'), '/') AS relative_path
                FROM Files
                WHERE domain = ?1
            )
            SELECT
                COALESCE(MAX(CASE WHEN relative_path = ?2 THEN 1 ELSE 0 END), 0) AS has_chat_storage,
                COALESCE(MAX(CASE WHEN relative_path = ?3 THEN 1 ELSE 0 END), 0) AS has_contacts,
                COALESCE(SUM(
                    CASE
                        WHEN relative_path LIKE 'Message/%'
                          OR relative_path LIKE 'Media/%'
                          OR relative_path LIKE 'stickers/%'
                        THEN 1
                        ELSE 0
                    END
                ), 0) AS media_file_count
            FROM whatsapp_files
            "#,
            (
                WHATSAPP_SHARED_DOMAIN,
                CHAT_STORAGE_RELATIVE_PATH,
                CONTACTS_RELATIVE_PATH,
            ),
            |row| {
                let has_chat_storage = row.get::<_, i64>(0)? > 0;
                let has_contacts = row.get::<_, i64>(1)? > 0;
                let media_file_count = row.get::<_, i64>(2)?;

                Ok(WhatsappManifestSummary {
                    has_chat_storage,
                    has_contacts,
                    media_file_count: usize::try_from(media_file_count.max(0))
                        .unwrap_or(usize::MAX),
                })
            },
        )
        .map_err(IphoneBackupError::from)
}

pub fn find_whatsapp_manifest_files<P>(_manifest_db_path: P) -> Result<WhatsappManifestFiles>
where
    P: AsRef<Path>,
{
    let files = read_manifest_files(_manifest_db_path)?;
    let mut chat_storage = None;
    let mut contacts = None;
    let mut media = Vec::new();

    for file in files {
        if !is_whatsapp_manifest_domain(&file.domain) {
            continue;
        }

        match file.relative_path.as_str() {
            CHAT_STORAGE_RELATIVE_PATH => chat_storage = Some(file),
            CONTACTS_RELATIVE_PATH => contacts = Some(file),
            _ if is_whatsapp_media_path(&file.relative_path) => media.push(file),
            _ => {}
        }
    }

    Ok(WhatsappManifestFiles {
        chat_storage,
        contacts,
        media,
    })
}

fn open_manifest_read_only<P>(manifest_db_path: P) -> Result<Connection>
where
    P: AsRef<Path>,
{
    Ok(Connection::open_with_flags(
        manifest_db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?)
}

fn is_whatsapp_manifest_domain(domain: &str) -> bool {
    domain == WHATSAPP_SHARED_DOMAIN
}

fn is_whatsapp_media_path(relative_path: &str) -> bool {
    let normalized = normalize_manifest_relative_path(relative_path);

    normalized.starts_with("Message/")
        || normalized.starts_with("Media/")
        || normalized.starts_with("stickers/")
}

fn optional_child_path(parent: &Path, child: &str) -> Option<PathBuf> {
    let path = parent.join(child);
    path.is_file().then_some(path)
}

fn backup_candidate_from_directory(path: &Path) -> BackupCandidate {
    let manifest_db_path = path.join("Manifest.db");
    let info_plist_path = optional_child_path(path, "Info.plist");
    let manifest_plist_path = optional_child_path(path, "Manifest.plist");
    let status_plist_path = optional_child_path(path, "Status.plist");
    let id = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "selected-backup".to_owned());

    BackupCandidate {
        id,
        path: path_to_string(path),
        manifest_db_path: path_to_string(&manifest_db_path),
        manifest_plist_path: manifest_plist_path.map(|path| path_to_string(&path)),
        info_plist_path: info_plist_path.map(|path| path_to_string(&path)),
        status_plist_path: status_plist_path.map(|path| path_to_string(&path)),
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn normalize_manifest_relative_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");

    normalized.trim().trim_start_matches('/').to_owned()
}

fn whatsapp_media_manifest_relative_path_candidates(path: &str) -> Vec<String> {
    let normalized = normalize_manifest_relative_path(path);
    if normalized.is_empty() {
        return Vec::new();
    }

    let mut candidates = vec![normalized.clone()];
    if !normalized.starts_with("Message/") && normalized.contains('/') {
        candidates.push(format!("Message/{normalized}"));
    }

    candidates
}

fn plist_string(root: &plist::Value, key: &str) -> Option<String> {
    root.as_dictionary()?
        .get(key)?
        .as_string()
        .map(ToOwned::to_owned)
        .filter(|value| !value.trim().is_empty())
}

fn plist_bool(root: &plist::Value, key: &str) -> Option<bool> {
    root.as_dictionary()?.get(key)?.as_boolean()
}

fn plist_date_string(root: &plist::Value, key: &str) -> Option<String> {
    let plist::Value::Date(date) = root.as_dictionary()?.get(key)? else {
        return None;
    };

    Some(date.to_xml_format())
}

fn backup_roots_from_env(
    home: Option<PathBuf>,
    user_profile: Option<PathBuf>,
    app_data: Option<PathBuf>,
) -> Vec<PathBuf> {
    backup_roots_from_env_for_platform(current_backup_platform(), home, user_profile, app_data)
}

fn backup_roots_from_env_for_platform(
    platform: BackupPlatform,
    home: Option<PathBuf>,
    user_profile: Option<PathBuf>,
    app_data: Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    match platform {
        BackupPlatform::Macos => {
            let Some(home) = home else {
                return roots;
            };
            roots.push(join_suffix(home, MACOS_BACKUP_SUFFIX));
        }
        BackupPlatform::Windows => {
            if let Some(user_profile) = user_profile {
                roots.push(join_suffix(user_profile, WINDOWS_STORE_BACKUP_SUFFIX));
            }
            if let Some(app_data) = app_data {
                roots.push(join_suffix(app_data, WINDOWS_LEGACY_BACKUP_SUFFIX));
            }
        }
        BackupPlatform::Unsupported => {
            return roots;
        }
    }

    roots
}

fn current_backup_platform() -> BackupPlatform {
    if cfg!(target_os = "macos") {
        BackupPlatform::Macos
    } else if cfg!(target_os = "windows") {
        BackupPlatform::Windows
    } else {
        BackupPlatform::Unsupported
    }
}

fn join_suffix(mut root: PathBuf, suffix: &[&str]) -> PathBuf {
    for part in suffix {
        root.push(part);
    }
    root
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        backup_roots_from_env_for_platform, join_suffix, BackupPlatform, MACOS_BACKUP_SUFFIX,
        WINDOWS_LEGACY_BACKUP_SUFFIX, WINDOWS_STORE_BACKUP_SUFFIX,
    };

    #[test]
    fn macos_backup_suffix_matches_apple_documented_path() {
        assert_eq!(
            join_suffix(PathBuf::from("/Users/example"), MACOS_BACKUP_SUFFIX),
            PathBuf::from("/Users/example/Library/Application Support/MobileSync/Backup")
        );
    }

    #[test]
    fn windows_backup_suffixes_cover_store_and_legacy_locations() {
        assert_eq!(
            join_suffix(
                PathBuf::from("C:/Users/example"),
                WINDOWS_STORE_BACKUP_SUFFIX
            ),
            PathBuf::from("C:/Users/example/Apple/MobileSync/Backup")
        );
        assert_eq!(
            join_suffix(
                PathBuf::from("C:/Users/example/AppData/Roaming"),
                WINDOWS_LEGACY_BACKUP_SUFFIX
            ),
            PathBuf::from("C:/Users/example/AppData/Roaming/Apple Computer/MobileSync/Backup")
        );
    }

    #[test]
    fn macos_backup_roots_use_home_only() {
        assert_eq!(
            backup_roots_from_env_for_platform(
                BackupPlatform::Macos,
                Some(PathBuf::from("/Users/example")),
                Some(PathBuf::from("C:/Users/example")),
                Some(PathBuf::from("C:/Users/example/AppData/Roaming")),
            ),
            vec![PathBuf::from(
                "/Users/example/Library/Application Support/MobileSync/Backup"
            )]
        );
    }

    #[test]
    fn windows_backup_roots_cover_store_and_legacy_installers() {
        assert_eq!(
            backup_roots_from_env_for_platform(
                BackupPlatform::Windows,
                Some(PathBuf::from("/Users/example")),
                Some(PathBuf::from("C:/Users/example")),
                Some(PathBuf::from("C:/Users/example/AppData/Roaming")),
            ),
            vec![
                PathBuf::from("C:/Users/example/Apple/MobileSync/Backup"),
                PathBuf::from("C:/Users/example/AppData/Roaming/Apple Computer/MobileSync/Backup"),
            ]
        );
    }

    #[test]
    fn backup_roots_are_empty_when_required_platform_env_is_missing() {
        assert!(backup_roots_from_env_for_platform(
            BackupPlatform::Macos,
            None,
            Some(PathBuf::from("C:/Users/example")),
            Some(PathBuf::from("C:/Users/example/AppData/Roaming")),
        )
        .is_empty());

        assert!(backup_roots_from_env_for_platform(
            BackupPlatform::Windows,
            Some(PathBuf::from("/Users/example")),
            None,
            None,
        )
        .is_empty());
    }

    #[test]
    fn unsupported_platforms_have_no_default_backup_roots() {
        assert!(backup_roots_from_env_for_platform(
            BackupPlatform::Unsupported,
            Some(PathBuf::from("/Users/example")),
            Some(PathBuf::from("C:/Users/example")),
            Some(PathBuf::from("C:/Users/example/AppData/Roaming")),
        )
        .is_empty());
    }
}
