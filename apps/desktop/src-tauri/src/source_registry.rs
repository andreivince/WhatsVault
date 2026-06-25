use std::{collections::HashMap, path::PathBuf, sync::Mutex};

pub(crate) type SourceRegistryState = Mutex<SourceRegistry>;

#[derive(Debug, Default)]
pub(crate) struct SourceRegistry {
    backup_paths: HashMap<String, PathBuf>,
    export_paths: HashMap<String, PathBuf>,
    next_export_handle: u64,
}

impl SourceRegistry {
    pub(crate) fn clear_backups(&mut self) {
        self.backup_paths.clear();
    }

    pub(crate) fn register_backup(&mut self, index: usize, path: PathBuf) -> String {
        let handle = format!("backup-source-{}", index + 1);
        self.backup_paths.insert(handle.clone(), path);
        handle
    }

    pub(crate) fn register_export(&mut self, path: PathBuf) -> String {
        self.next_export_handle += 1;
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
