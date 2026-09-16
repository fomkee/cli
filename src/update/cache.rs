use crate::error::CliError;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use tempfile::NamedTempFile;

/// Last discovery attempt and optional stable release version.
#[derive(Clone, Deserialize, Serialize)]
pub struct CacheEntry {
    pub checked_at: u64,
    pub latest_version: Option<String>,
}
/// Storage boundary for optional update discovery metadata.
pub trait UpdateCache {
    /// Read existing discovery metadata.
    fn load(&self) -> Result<Option<CacheEntry>, CliError>;
    /// Replace discovery metadata atomically.
    fn save(&self, entry: &CacheEntry) -> Result<(), CliError>;
}
/// File-backed cache that contains no credentials.
pub struct FileUpdateCache {
    path: PathBuf,
}
impl FileUpdateCache {
    /// Choose an update cache path within the CLI configuration directory.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub(super) fn try_lock(&self) -> Result<File, CliError> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| CliError::Configuration("update cache has no parent".into()))?;
        fs::create_dir_all(parent)
            .map_err(|error| CliError::io("cannot create update cache directory", error))?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.path.with_extension("lock"))
            .map_err(|error| CliError::io("cannot open update cache lock", error))?;
        FileExt::try_lock_exclusive(&lock)
            .map_err(|error| CliError::io("another update check is running", error))?;
        Ok(lock)
    }
}
impl UpdateCache for FileUpdateCache {
    fn load(&self) -> Result<Option<CacheEntry>, CliError> {
        let file = match fs::File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(CliError::io("cannot read update cache", error)),
        };
        let mut bytes = Vec::new();
        file.take(4096)
            .read_to_end(&mut bytes)
            .map_err(|error| CliError::io("cannot read update cache", error))?;
        // Damaged optional cache data is a cache miss, never a configuration failure.
        Ok(serde_json::from_slice(&bytes).ok())
    }
    fn save(&self, entry: &CacheEntry) -> Result<(), CliError> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| CliError::Configuration("update cache has no parent".into()))?;
        fs::create_dir_all(parent)
            .map_err(|error| CliError::io("cannot create update cache directory", error))?;
        let mut file = NamedTempFile::new_in(parent)
            .map_err(|error| CliError::io("cannot stage update cache", error))?;
        serde_json::to_writer(&mut file, entry)
            .map_err(|_| CliError::Configuration("cannot encode update cache".into()))?;
        file.flush()
            .and_then(|()| file.as_file().sync_all())
            .map_err(|error| CliError::io("cannot write update cache", error))?;
        file.persist(&self.path)
            .map_err(|error| CliError::io("cannot replace update cache", error.error))?;
        Ok(())
    }
}
/// Reusable isolated cache for deterministic update checks.
#[derive(Default)]
pub struct InMemoryUpdateCache {
    entry: RefCell<Option<CacheEntry>>,
}
impl UpdateCache for InMemoryUpdateCache {
    fn load(&self) -> Result<Option<CacheEntry>, CliError> {
        self.entry
            .try_borrow()
            .map(|entry| entry.clone())
            .map_err(|_| CliError::Configuration("update cache is already borrowed".into()))
    }
    fn save(&self, entry: &CacheEntry) -> Result<(), CliError> {
        *self
            .entry
            .try_borrow_mut()
            .map_err(|_| CliError::Configuration("update cache is already borrowed".into()))? =
            Some(entry.clone());
        Ok(())
    }
}
