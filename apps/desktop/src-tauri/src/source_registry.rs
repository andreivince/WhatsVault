use std::{collections::HashMap, path::PathBuf, sync::Mutex};

pub(crate) type SourceRegistryState = Mutex<SourceRegistry>;

#[derive(Debug, Default)]
pub(crate) struct SourceRegistry {
    backup_paths: HashMap<String, PathBuf>,
    export_paths: HashMap<String, PathBuf>,
    next_backup_handle: u64,
    next_export_handle: u64,
}

impl SourceRegistry {
    pub(crate) fn clear_backups(&mut self) {
        self.backup_paths.clear();
    }

    pub(crate) fn register_backup(&mut self, path: PathBuf) -> String {
        self.next_backup_handle = self.next_backup_handle.saturating_add(1);
        let handle = format!("backup-source-{}", self.next_backup_handle);
        self.backup_paths.insert(handle.clone(), path);
        handle
    }

    pub(crate) fn register_export(&mut self, path: PathBuf) -> String {
        self.export_paths.clear();
        self.next_export_handle = self.next_export_handle.saturating_add(1);
        let handle = format!("export-source-{}", self.next_export_handle);
        self.export_paths.insert(handle.clone(), path);
        handle
    }

    pub(crate) fn backup_path(&self, handle: &str) -> Option<PathBuf> {
        self.backup_paths.get(handle).cloned()
    }

    pub(crate) fn export_path(&self, handle: &str) -> Option<PathBuf> {
        self.export_paths.get(handle).cloned()
    }
}

pub(crate) fn registered_backup_path(
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

pub(crate) fn registered_export_path(
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::SourceRegistry;

    #[test]
    fn refreshed_backups_receive_new_opaque_handles() {
        let mut registry = SourceRegistry::default();
        let first_handle = registry.register_backup(PathBuf::from("first-backup"));

        registry.clear_backups();
        let refreshed_handle = registry.register_backup(PathBuf::from("refreshed-backup"));

        assert_ne!(first_handle, refreshed_handle);
        assert_eq!(registry.backup_path(&first_handle), None);
        assert_eq!(
            registry.backup_path(&refreshed_handle),
            Some(PathBuf::from("refreshed-backup"))
        );
    }

    #[test]
    fn registering_an_export_retires_the_previous_export_path() {
        let mut registry = SourceRegistry::default();
        let first_handle = registry.register_export(PathBuf::from("first-export.zip"));
        let active_handle = registry.register_export(PathBuf::from("active-export.zip"));

        assert_eq!(registry.export_path(&first_handle), None);
        assert_eq!(
            registry.export_path(&active_handle),
            Some(PathBuf::from("active-export.zip"))
        );
    }
}
