use crate::commands::Cli;
use clap::Parser;
fn creation(extra: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(
        ["fomkeecli", "maintenance", "create"]
            .into_iter()
            .chain(extra.iter().copied()),
    )
}
#[test]
fn test_rejects_missing_monitor_selection() {
    // Act
    let result = creation(&[
        "--title",
        "Upgrade",
        "--start",
        "2030-01-01T09:00:00Z",
        "--end",
        "2030-01-01T10:00:00Z",
    ]);
    // Assert
    assert!(result.is_err());
}
#[test]
fn test_rejects_timestamp_without_explicit_timezone() {
    // Act
    let result = creation(&[
        "--title",
        "Upgrade",
        "--start",
        "2030-01-01T09:00:00",
        "--end",
        "2030-01-01T10:00:00Z",
        "--monitor",
        "00000000-0000-0000-0000-000000000001",
    ]);
    // Assert
    assert!(result.is_err());
}
#[test]
fn test_rejects_combining_complete_file_with_partial_flags() {
    // Act
    let result = creation(&["--file", "-", "--description", "Override"]);
    // Assert
    assert!(result.is_err());
}
#[test]
fn test_requires_exactly_one_public_note_source() {
    // Act
    let result = Cli::try_parse_from([
        "fomkeecli",
        "incident",
        "post",
        "00000000-0000-0000-0000-000000000001",
        "--message",
        "Update",
        "--message-file",
        "-",
    ]);
    // Assert
    assert!(result.is_err());
}
