use std::fs::File;
use std::io;
use std::os::windows::fs::MetadataExt;

use windows_permissions::constants::{SeObjectType, SecurityInformation};
use windows_permissions::{LocalBox, SecurityDescriptor, wrappers};

use super::unsafe_file;

fn owner_descriptor(file: &File) -> io::Result<LocalBox<SecurityDescriptor>> {
    let descriptor = wrappers::GetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner,
    )?;
    let owner = wrappers::GetSecurityDescriptorOwner(&descriptor)?.ok_or_else(unsafe_file)?;
    let sid = wrappers::ConvertSidToStringSid(owner)?;
    let sid = sid.to_str().ok_or_else(unsafe_file)?;
    format!("D:P(A;;FA;;;{sid})").parse()
}

pub(super) fn protect(file: &mut File) -> io::Result<()> {
    let descriptor = owner_descriptor(file)?;
    let dacl = wrappers::GetSecurityDescriptorDacl(&descriptor)?.ok_or_else(unsafe_file)?;
    wrappers::SetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
        None,
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
    let expected = owner_descriptor(file)?;
    let actual = wrappers::GetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Dacl,
    )?;
    let expected = wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(
        &expected,
        SecurityInformation::Dacl,
    )?;
    let actual = wrappers::ConvertSecurityDescriptorToStringSecurityDescriptor(
        &actual,
        SecurityInformation::Dacl,
    )?;
    if actual != expected {
        return Err(unsafe_file());
    }
    Ok(())
}
