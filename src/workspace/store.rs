use std::cell::RefCell;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use fs2::FileExt;
use tempfile::NamedTempFile;

use super::model::WorkspaceRegistry;
use super::private_files;
use crate::error::CliError;

/// Storage for non-secret workspace connection metadata.
pub trait WorkspaceStore {
    /// Load a consistent registry, migrating legacy metadata if supported.
    fn load(&self) -> Result<WorkspaceRegistry, CliError>;
    /// Replace the registry atomically or return a configuration error.
    fn save(&self, registry: &WorkspaceRegistry) -> Result<(), CliError>;
}

/// Serializes CLI profile operations with a file lock and saves metadata atomically.
pub struct FileWorkspaceStore {
    directory: PathBuf,
    _lock: File,
}

impl FileWorkspaceStore {
    /// Return the directory whose registry is locked by this store.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Acquire the write lock without waiting; release it when the store drops.
    pub fn open(directory: PathBuf) -> Result<Self, CliError> {
        private_files::create_directory(&directory).map_err(storage_error)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join("workspaces.lock"))
            .map_err(storage_error)?;
        FileExt::try_lock_exclusive(&lock).map_err(|_| CliError::Configuration(
            "another CLI process is using the workspace configuration; try again when it finishes".into()
        ))?;
        Ok(Self {
            directory,
            _lock: lock,
        })
    }

    /// Open the environment-selected or platform-default configuration directory.
    pub fn from_environment() -> Result<Self, CliError> {
        Self::open(config_directory(None)?)
    }
}

/// Resolve configuration paths without creating files or accessing credentials.
pub fn config_directory(explicit: Option<&Path>) -> Result<PathBuf, CliError> {
    let directory = match explicit
        .map(Path::as_os_str)
        .map(ToOwned::to_owned)
        .or_else(|| env::var_os("FOMKEE_CONFIG_DIR"))
    {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        Some(_) => {
            return Err(CliError::Configuration(
                "FOMKEE_CONFIG_DIR cannot be empty".into(),
            ));
        }
        None => ProjectDirs::from("", "", "fomkeecli")
            .ok_or_else(|| {
                CliError::Configuration(
                    "cannot locate configuration directory; set FOMKEE_CONFIG_DIR".into(),
                )
            })?
            .config_dir()
            .to_owned(),
    };
    if directory.is_absolute() {
        Ok(directory)
    } else {
        Ok(env::current_dir().map_err(storage_error)?.join(directory))
    }
}

impl WorkspaceStore for FileWorkspaceStore {
    fn load(&self) -> Result<WorkspaceRegistry, CliError> {
        let (registry, legacy) = load_registry(&self.directory)?;
        if legacy {
            self.save(&registry)?;
        }
        Ok(registry)
    }

    fn save(&self, registry: &WorkspaceRegistry) -> Result<(), CliError> {
        let mut temporary = NamedTempFile::new_in(&self.directory).map_err(storage_error)?;
        let contents = toml::to_string_pretty(registry).map_err(|_| {
            CliError::Configuration("cannot serialize workspace configuration".into())
        })?;
        temporary
            .write_all(contents.as_bytes())
            .map_err(storage_error)?;
        temporary.flush().map_err(storage_error)?;
        temporary.as_file().sync_all().map_err(storage_error)?;
        temporary
            .persist(self.directory.join("workspaces.toml"))
            .map_err(|error| storage_error(error.error))?;
        Ok(())
    }
}

/// Read metadata without locks, file creation, migration, or credential access.
pub fn read_registry(directory: &Path) -> Result<WorkspaceRegistry, CliError> {
    load_registry(directory).map(|(registry, _)| registry)
}

fn load_registry(directory: &Path) -> Result<(WorkspaceRegistry, bool), CliError> {
    let (registry, legacy): (WorkspaceRegistry, bool) =
        match fs::read_to_string(directory.join("workspaces.toml")) {
            Ok(contents) => (
                toml::from_str(&contents).map_err(|_| {
                    CliError::Configuration(
                        "cannot parse workspaces.toml; repair it before continuing".into(),
                    )
                })?,
                false,
            ),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let file = match File::open(directory.join("workspaces.json")) {
                    Ok(file) => file,
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        return Ok((WorkspaceRegistry::default(), false));
                    }
                    Err(error) => return Err(storage_error(error)),
                };
                (
                    serde_json::from_reader(file).map_err(|_| {
                        CliError::Configuration(
                            "cannot parse legacy workspaces.json; repair it before continuing"
                                .into(),
                        )
                    })?,
                    true,
                )
            }
            Err(error) => return Err(storage_error(error)),
        };
    if registry.version != 1
        || registry
            .active
            .as_ref()
            .is_some_and(|alias| !registry.workspaces.contains_key(alias))
    {
        return Err(CliError::Configuration(
            "unsupported or inconsistent workspace configuration".into(),
        ));
    }
    Ok((registry, legacy))
}

fn storage_error(error: io::Error) -> CliError {
    CliError::Configuration(format!(
        "cannot access workspace configuration: {}",
        error.kind()
    ))
}

/// Reusable isolated workspace registry.
#[derive(Default)]
pub struct InMemoryWorkspaceStore {
    registry: RefCell<WorkspaceRegistry>,
}

impl WorkspaceStore for InMemoryWorkspaceStore {
    fn load(&self) -> Result<WorkspaceRegistry, CliError> {
        self.registry
            .try_borrow()
            .map(|registry| registry.clone())
            .map_err(|_| CliError::Configuration("workspace registry is busy".into()))
    }

    fn save(&self, registry: &WorkspaceRegistry) -> Result<(), CliError> {
        *self
            .registry
            .try_borrow_mut()
            .map_err(|_| CliError::Configuration("workspace registry is busy".into()))? =
            registry.clone();
        Ok(())
    }
}
