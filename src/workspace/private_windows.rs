use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};

use windows_permissions::constants::{AccessRights, SeObjectType, SecurityInformation};
use windows_permissions::utilities::current_process_sid;
use windows_permissions::{LocalBox, SecurityDescriptor, Sid, wrappers};

use super::unsafe_file;

fn owner_descriptor(owner: &Sid) -> io::Result<LocalBox<SecurityDescriptor>> {
    let sid = wrappers::ConvertSidToStringSid(owner)?;
    let sid = sid.to_str().ok_or_else(unsafe_file)?;
    format!("D:P(A;;FA;;;{sid})").parse()
}

pub(super) fn request_protection_access(options: &mut OpenOptions, write: bool) {
    let mut access = AccessRights::GenericRead
        | AccessRights::ReadControl
        | AccessRights::WriteDac
        | AccessRights::WriteOwner;
    if write {
        access |= AccessRights::GenericWrite;
    }
    options.access_mode(access.bits());
}

pub(super) fn protect(file: &mut File) -> io::Result<()> {
    let owner = current_process_sid()?;
    let descriptor = owner_descriptor(&owner)?;
    let dacl = wrappers::GetSecurityDescriptorDacl(&descriptor)?.ok_or_else(unsafe_file)?;
    wrappers::SetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
        Some(&owner),
        None,
        Some(dacl),
        None,
    )?;
    validate(file)
}

pub(super) fn validate(file: &File) -> io::Result<()> {
    if file.metadata()?.file_attributes() & 0x400 != 0 {
        return Err(unsafe_file());
    }
    let actual = wrappers::GetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl,
    )?;
    let current_user = current_process_sid()?;
    validate_descriptor(&actual, &current_user)
}

fn validate_descriptor(actual: &SecurityDescriptor, current_user: &Sid) -> io::Result<()> {
    let owner = wrappers::GetSecurityDescriptorOwner(actual)?.ok_or_else(unsafe_file)?;
    if !wrappers::EqualSid(owner, current_user) {
        return Err(unsafe_file());
    }
    let expected = owner_descriptor(current_user)?;
    let expected = wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(
        &expected,
        SecurityInformation::Dacl,
    )?;
    let actual = wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(
        actual,
        SecurityInformation::Dacl,
    )?;
    if !protected_dacl_matches(
        actual.to_str().ok_or_else(unsafe_file)?,
        expected.to_str().ok_or_else(unsafe_file)?,
    ) {
        return Err(unsafe_file());
    }
    Ok(())
}

fn protected_dacl_matches(actual: &str, expected: &str) -> bool {
    // AI records inheritance history; P still blocks inheritance. ACEs must match exactly.
    actual == expected
        || actual
            .strip_prefix("D:PAI")
            .zip(expected.strip_prefix("D:P"))
            .is_some_and(|(actual, expected)| actual == expected)
}

#[cfg(test)]
#[path = "private_windows_tests.rs"]
mod tests;
