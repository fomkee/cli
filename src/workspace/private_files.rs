use std::fs::{DirBuilder, File, OpenOptions};
use std::io;
use std::path::Path;
use tempfile::NamedTempFile;

#[cfg(windows)]
#[path = "private_windows.rs"]
mod windows;

pub(super) fn create_directory(path: &Path) -> io::Result<()> {
    let mut builder = DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

pub(super) fn open(path: &Path, create: bool) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(create).create_new(create);
    configure(&mut options);
    #[cfg(windows)]
    if create {
        windows::request_protection_access(&mut options, true);
    }
    let mut file = options.open(path)?;
    if create {
        protect(&mut file)?;
    }
    validate(&file)?;
    Ok(file)
}

pub(super) fn protect_temporary(temporary: &mut NamedTempFile) -> io::Result<()> {
    #[cfg(unix)]
    protect(temporary.as_file_mut())?;
    #[cfg(windows)]
    {
        let mut options = OpenOptions::new();
        options.read(true);
        windows::request_protection_access(&mut options, false);
        configure(&mut options);
        let mut file = options.open(temporary.path())?;
        protect(&mut file)?;
    }
    validate(temporary.as_file())
}

#[cfg(unix)]
fn configure(options: &mut OpenOptions) {
    use rustix::fs::OFlags;
    use std::os::unix::fs::OpenOptionsExt;
    options
        .mode(0o600)
        .custom_flags((OFlags::NOFOLLOW | OFlags::NONBLOCK).bits() as i32);
}

#[cfg(windows)]
fn configure(options: &mut OpenOptions) {
    use std::os::windows::fs::OpenOptionsExt;
    // Open reparse points themselves so validation can reject them before reading.
    options.custom_flags(0x00200000);
}

#[cfg(unix)]
pub(super) fn protect(file: &mut File) -> io::Result<()> {
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(Permissions::from_mode(0o600))
}

#[cfg(windows)]
pub(super) fn protect(file: &mut File) -> io::Result<()> {
    windows::protect(file)
}

pub(super) fn validate(file: &File) -> io::Result<()> {
    if !file.metadata()?.is_file() {
        return Err(unsafe_file());
    }
    validate_permissions(file)
}

#[cfg(unix)]
fn validate_permissions(file: &File) -> io::Result<()> {
    use rustix::process::geteuid;
    use std::os::unix::fs::MetadataExt;
    let metadata = file.metadata()?;
    if metadata.mode() & 0o077 != 0 || metadata.uid() != geteuid().as_raw() || metadata.nlink() != 1
    {
        return Err(unsafe_file());
    }
    Ok(())
}

#[cfg(windows)]
fn validate_permissions(file: &File) -> io::Result<()> {
    windows::validate(file)
}

pub(super) fn unsafe_file() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "credential file must be a private regular file owned by this user",
    )
}
