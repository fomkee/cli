#![cfg(test)]

#[path = "skill_export/support.rs"]
mod support;

use std::fs;

use serde_json::Value;
use support::{
    Fixture, assert_manifest_matches_export, assert_matches_sources, failure, files, stdout,
};

#[test]
fn test_exports_every_bundled_file_byte_for_byte_without_credentials() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    stdout(&mut fixture.export());

    // Assert
    assert_matches_sources(&fixture.destination());
    assert!(!fixture.config().exists());
}

#[test]
fn test_human_output_explains_review_and_native_import() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    let output = stdout(&mut fixture.export());

    // Assert
    assert!(output.contains("Exported Fomkee skills"));
    assert!(output.contains("managing-fomkee-monitors"));
    assert!(output.contains("writing-fomkee-check-functions"));
    assert!(output.contains("Review each SKILL.md"));
    assert!(output.contains("No agent configuration was changed."));
    assert!(output.contains(&fixture.destination().display().to_string()));
    assert!(!output.contains('\u{1b}'));
}

#[test]
fn test_json_returns_the_version_and_complete_export_manifest_without_color() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    let output = stdout(fixture.export().args(["--json", "--color", "always"]));

    // Assert
    assert_manifest_matches_export(&output, &fixture.destination());
    assert!(!output.contains('\u{1b}'));
}

#[test]
fn test_explicit_human_color_is_supported() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    let output = stdout(fixture.export().args(["--color", "always"]));

    // Assert
    assert!(output.contains('\u{1b}'));
    assert_matches_sources(&fixture.destination());
}

#[test]
fn test_relative_destination_is_reported_as_an_absolute_path() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    let output =
        stdout(
            fixture
                .command()
                .args(["skill", "export", "review skills", "--output", "json"]),
        );

    // Assert
    assert_manifest_matches_export(&output, &fixture.destination());
}

#[test]
fn test_export_does_not_resolve_config_or_authenticate() {
    // Arrange
    let fixture = Fixture::new();
    fs::write(fixture.config(), "not a configuration directory").unwrap();

    // Act
    stdout(
        fixture
            .export()
            .env("FOMKEE_API_TOKEN", "invalid\ntoken")
            .env("FOMKEE_API_URL", "invalid origin")
            .args(["--workspace", "missing"]),
    );

    // Assert
    assert_matches_sources(&fixture.destination());
    assert_eq!(
        fs::read_to_string(fixture.config()).unwrap(),
        "not a configuration directory"
    );
}

#[test]
fn test_existing_empty_destination_is_rejected() {
    // Arrange
    let fixture = Fixture::new();
    fs::create_dir(fixture.destination()).unwrap();

    // Act
    let error = failure(&mut fixture.export(), 2);

    // Assert
    assert!(error.contains("choose a new directory"));
    assert!(files(&fixture.destination()).is_empty());
}

#[test]
fn test_existing_file_is_not_overwritten() {
    // Arrange
    let fixture = Fixture::new();
    fs::write(fixture.destination(), "user content").unwrap();

    // Act
    failure(&mut fixture.export(), 2);

    // Assert
    assert_eq!(
        fs::read_to_string(fixture.destination()).unwrap(),
        "user content"
    );
}

#[test]
fn test_repeated_export_preserves_user_edits_and_reports_a_json_error() {
    // Arrange
    let fixture = Fixture::new();
    stdout(&mut fixture.export());
    fs::write(
        fixture
            .destination()
            .join("managing-fomkee-monitors/SKILL.md"),
        "reviewed by user",
    )
    .unwrap();
    let before = files(&fixture.destination());

    // Act
    let error = failure(fixture.export().arg("--json"), 2);

    // Assert
    assert_eq!(
        serde_json::from_str::<Value>(&error)
            .unwrap()
            .get("category")
            .unwrap(),
        "validation"
    );
    assert_eq!(files(&fixture.destination()), before);
}

#[test]
fn test_missing_parent_is_reported_without_creating_directories() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    let error = failure(
        fixture
            .command()
            .args(["skill", "export", "missing/review", "--json"]),
        7,
    );

    // Assert
    assert_eq!(
        serde_json::from_str::<Value>(&error)
            .unwrap()
            .get("category")
            .unwrap(),
        "filesystem"
    );
    assert!(error.contains("existing parent directory"));
    assert!(!fixture.root.path().join("missing").exists());
}

#[test]
fn test_destination_is_required() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    let error = failure(fixture.command().args(["skill", "export"]), 2);

    // Assert
    assert!(error.contains("<DIRECTORY>"));
    assert!(files(fixture.root.path()).is_empty());
}

#[test]
fn test_help_explains_offline_non_overwriting_export() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    let output = stdout(fixture.command().args(["skill", "export", "--help"]));

    // Assert
    assert!(output.contains("offline"));
    assert!(output.contains("parent must already exist"));
    assert!(files(fixture.root.path()).is_empty());
}

#[test]
fn test_completion_discovers_the_skill_export_command() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    let output = stdout(fixture.command().args(["completion", "bash"]));

    // Assert
    assert!(output.contains("fomkeecli__subcmd__skill__subcmd__export"));
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn test_destination_symlink_is_rejected_without_writing_its_target() {
        // Arrange
        let fixture = Fixture::new();
        let target = fixture.root.path().join("target");
        fs::create_dir(&target).unwrap();
        symlink(&target, fixture.destination()).unwrap();

        // Act
        failure(&mut fixture.export(), 2);

        // Assert
        assert!(files(&target).is_empty());
    }

    #[test]
    fn test_dangling_destination_symlink_is_rejected() {
        // Arrange
        let fixture = Fixture::new();
        let target = fixture.root.path().join("missing-target");
        symlink(&target, fixture.destination()).unwrap();

        // Act
        failure(&mut fixture.export(), 2);

        // Assert
        assert!(!target.exists());
    }
}
