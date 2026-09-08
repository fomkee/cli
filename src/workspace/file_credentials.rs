use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;

use fs2::FileExt;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use super::credentials::CredentialStore;
use super::model::CredentialId;
use super::private_files;
use crate::config::ApiToken;
use crate::error::CliError;

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Credentials {
    tokens: BTreeMap<CredentialId, String>,
}

/// Local plaintext TOML credentials protected by owner-only filesystem permissions.
pub struct FileCredentialStore {
    directory: PathBuf,
}

impl FileCredentialStore {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    fn lock(&self) -> Result<File, CliError> {
        private_files::create_directory(&self.directory).map_err(storage_error)?;
        self.validate_directory()?;
        let path = self.directory.join("credentials.lock");
        let file = match private_files::open(&path, true) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                private_files::open(&path, false).map_err(storage_error)?
            }
            Err(error) => return Err(storage_error(error)),
        };
        FileExt::try_lock_exclusive(&file).map_err(|_| {
            CliError::Configuration(
                "credential file is busy; retry after the other CLI process finishes".into(),
            )
        })?;
        Ok(file)
    }

    fn validate_directory(&self) -> Result<(), CliError> {
        let metadata = fs::symlink_metadata(&self.directory).map_err(storage_error)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(storage_error(private_files::unsafe_file()));
        }
        #[cfg(unix)]
        {
            use rustix::process::geteuid;
            use std::os::unix::fs::MetadataExt;
            if metadata.mode() & 0o022 != 0 || metadata.uid() != geteuid().as_raw() {
                return Err(storage_error(private_files::unsafe_file()));
            }
        }
        Ok(())
    }

    fn load(&self) -> Result<Credentials, CliError> {
        let mut file = match private_files::open(&self.directory.join("credentials.toml"), false) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(Credentials::default());
            }
            Err(error) => return Err(storage_error(error)),
        };
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(storage_error)?;
        toml::from_str(&contents).map_err(|_| {
            CliError::Configuration(
                "cannot parse credentials.toml; repair it before continuing (contents withheld)"
                    .into(),
            )
        })
    }

    fn save(&self, credentials: &Credentials) -> Result<(), CliError> {
        let mut temporary = NamedTempFile::new_in(&self.directory).map_err(storage_error)?;
        private_files::protect_temporary(&mut temporary).map_err(storage_error)?;
        let contents = toml::to_string_pretty(credentials).map_err(|_| {
            CliError::Configuration("cannot serialize credentials.toml (contents withheld)".into())
        })?;
        temporary
            .write_all(contents.as_bytes())
            .map_err(storage_error)?;
        temporary.flush().map_err(storage_error)?;
        temporary.as_file().sync_all().map_err(storage_error)?;
        temporary
            .persist(self.directory.join("credentials.toml"))
            .map_err(|error| storage_error(error.error))?;
        Ok(())
    }
}

impl CredentialStore for FileCredentialStore {
    fn get(&self, id: &CredentialId) -> Result<ApiToken, CliError> {
        let _lock = self.lock()?;
        let credentials = self.load()?;
        let token = credentials
            .tokens
            .get(id)
            .ok_or(CliError::MissingSavedCredential)?;
        ApiToken::new(token.clone())
    }

    fn set(&self, id: &CredentialId, token: &ApiToken) -> Result<(), CliError> {
        let _lock = self.lock()?;
        let mut credentials = self.load()?;
        credentials
            .tokens
            .insert(id.clone(), token.expose().to_owned());
        self.save(&credentials)
    }

    fn delete(&self, id: &CredentialId) -> Result<(), CliError> {
        let _lock = self.lock()?;
        let mut credentials = self.load()?;
        if credentials.tokens.remove(id).is_some() {
            self.save(&credentials)?;
        }
        Ok(())
    }
}

fn storage_error(error: io::Error) -> CliError {
    CliError::Configuration(format!(
        "cannot access protected credentials.toml: {}; check file ownership and owner-only permissions",
        error.kind()
    ))
}
