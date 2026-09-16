use crate::error::CliError;
use fs2::FileExt;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use tempfile::NamedTempFile;

/// Installation effect; callers must verify release identity and checksum first.
pub trait ExecutableInstaller {
    /// Replace the installed executable with verified bytes.
    fn install(&self, bytes: &[u8]) -> Result<(), CliError>;
}
/// Cross-platform replacement of this process's executable, preserving permissions.
pub struct RunningExecutable;
impl ExecutableInstaller for RunningExecutable {
    fn install(&self, bytes: &[u8]) -> Result<(), CliError> {
        let executable = env::current_exe()
            .and_then(|path| path.canonicalize())
            .map_err(|error| CliError::io("cannot locate the installed executable", error))?;
        let directory = executable.parent().ok_or_else(|| {
            CliError::Configuration("installed executable has no parent directory".into())
        })?;
        let lock = OpenOptions::new().read(true).write(true).create(true).truncate(false)
            .open(directory.join(".fomkeecli-update.lock")).map_err(|error| CliError::io("cannot write to the installation directory; install in a user-writable directory", error))?;
        FileExt::try_lock_exclusive(&lock)
            .map_err(|error| CliError::io("another self-update is running", error))?;
        let mut staged = NamedTempFile::new_in(directory)
            .map_err(|error| CliError::io("cannot stage executable update", error))?;
        staged
            .write_all(bytes)
            .and_then(|()| staged.as_file().sync_all())
            .map_err(|error| CliError::io("cannot write staged executable", error))?;
        let permissions = fs::metadata(&executable)
            .map_err(|error| CliError::io("cannot read executable permissions", error))?
            .permissions();
        staged
            .as_file()
            .set_permissions(permissions)
            .map_err(|error| CliError::io("cannot preserve executable permissions", error))?;
        self_replace::self_replace(staged.path()).map_err(|error| {
            CliError::io(
                "cannot replace executable; inspect the installation before retrying",
                error,
            )
        })
    }
}
