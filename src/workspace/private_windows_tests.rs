use super::*;
use tempfile::{NamedTempFile, tempdir};

fn descriptor(owner: &str, dacl: &str) -> LocalBox<SecurityDescriptor> {
    format!("O:{owner}D:{dacl}").parse().unwrap()
}

fn user_sid() -> String {
    wrappers::ConvertSidToStringSid(&current_process_sid().unwrap())
        .unwrap()
        .into_string()
        .unwrap()
}

fn validate_dacl(dacl: &str) -> io::Result<()> {
    validate_descriptor(
        &descriptor(&user_sid(), dacl),
        &current_process_sid().unwrap(),
    )
}

#[test]
fn test_accepts_current_user_only_protected_dacl() {
    // Arrange
    let dacl = format!("P(A;;FA;;;{})", user_sid());
    // Act
    let result = validate_dacl(&dacl);
    // Assert
    assert!(result.is_ok());
}

#[test]
fn test_accepts_auto_inherited_history_without_changing_protected_aces() {
    // Arrange
    let dacl = format!("PAI(A;;FA;;;{})", user_sid());
    // Act
    let result = validate_dacl(&dacl);
    // Assert
    assert!(result.is_ok());
}

#[test]
fn test_rejects_unprotected_dacl() {
    // Arrange
    let dacl = format!("AI(A;;FA;;;{})", user_sid());
    // Act
    let result = validate_dacl(&dacl);
    // Assert
    assert!(result.is_err());
}

#[test]
fn test_rejects_additional_everyone_read_permission() {
    // Arrange
    let dacl = format!("PAI(A;;FA;;;{})(A;;FR;;;WD)", user_sid());
    // Act
    let result = validate_dacl(&dacl);
    // Assert
    assert!(result.is_err());
}

#[test]
fn test_rejects_admin_group_instead_of_current_user() {
    // Arrange
    let dacl = "PAI(A;;FA;;;BA)";
    // Act
    let result = validate_dacl(dacl);
    // Assert
    assert!(result.is_err());
}

#[test]
fn test_rejects_different_owner_even_with_current_user_only_dacl() {
    // Arrange
    let descriptor = descriptor("BA", &format!("P(A;;FA;;;{})", user_sid()));
    // Act
    let result = validate_descriptor(&descriptor, &current_process_sid().unwrap());
    // Assert
    assert!(result.is_err());
}

#[test]
fn test_rejects_missing_dacl() {
    // Arrange
    let descriptor = format!("O:{}", user_sid())
        .parse::<LocalBox<SecurityDescriptor>>()
        .unwrap();
    // Act
    let result = validate_descriptor(&descriptor, &current_process_sid().unwrap());
    // Assert
    assert!(result.is_err());
}

#[test]
fn test_rejects_inherited_ace_even_when_dacl_is_protected() {
    // Arrange
    let dacl = format!("PAI(A;ID;FA;;;{})", user_sid());
    // Act
    let result = validate_dacl(&dacl);
    // Assert
    assert!(result.is_err());
}

#[test]
fn test_new_private_file_is_owned_by_current_user_and_can_be_reopened() {
    // Arrange
    let directory = tempdir().unwrap();
    let path = directory.path().join("private-file");
    // Act
    let file = super::super::open(&path, true).unwrap();
    // Assert
    assert_current_user_owner(&file);
    assert!(super::super::open(&path, false).is_ok());
}

#[test]
fn test_temporary_file_remains_private_after_persisting() {
    // Arrange
    let directory = tempdir().unwrap();
    let path = directory.path().join("credentials.toml");
    let mut temporary = NamedTempFile::new_in(directory.path()).unwrap();
    // Act
    super::super::protect_temporary(&mut temporary).unwrap();
    let file = temporary.persist(&path).unwrap();
    // Assert
    assert_current_user_owner(&file);
    assert!(super::super::open(&path, false).is_ok());
}

fn assert_current_user_owner(file: &File) {
    let descriptor = wrappers::GetSecurityInfo(
        file,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner,
    )
    .unwrap();
    let owner = wrappers::GetSecurityDescriptorOwner(&descriptor)
        .unwrap()
        .unwrap();
    assert!(wrappers::EqualSid(owner, &current_process_sid().unwrap()));
    assert!(validate(file).is_ok());
}
