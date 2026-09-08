#![cfg(test)]

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};

struct Fixture {
    directory: TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self {
            directory: tempdir().unwrap(),
        }
    }
    fn path(&self) -> PathBuf {
        self.directory.path().join("config")
    }
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_fomkeecli"));
        command
            .env("FOMKEE_CONFIG_DIR", self.path())
            .env_remove("FOMKEE_API_TOKEN")
            .env_remove("FOMKEE_API_URL")
            .env_remove("FOMKEE_PROFILE")
            .env_remove("FOMKEE_WORKSPACE_ID");
        command
    }
    fn saved_workspace(&self) {
        fs::create_dir(self.path()).unwrap();
        fs::write(
            self.path().join("workspaces.toml"),
            r#"
version = 1
active = "personal"
[workspaces.personal]
api_url = "https://primary.fomkee.dev"
workspace_id = "00000000-0000-0000-0000-000000000001"
name = "Personal"
slug = "personal"
credential_id = "00000000-0000-0000-0000-000000000002"
credential_store = "file"
"#,
        )
        .unwrap();
        fs::write(
            self.path().join("credentials.toml"),
            "fk_secret_never_read_this_invalid_toml",
        )
        .unwrap();
    }
}

fn stdout(output: Output) -> String {
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn test_config_paths_default_to_human_output_without_creating_configuration() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    let output = stdout(
        fixture
            .command()
            .args(["config", "paths"])
            .output()
            .unwrap(),
    );

    // Assert
    assert!(output.contains("workspaces.toml"));
    assert!(serde_json::from_str::<Value>(&output).is_err());
    assert!(!fixture.path().exists());
}

#[test]
fn test_json_flag_returns_exact_configuration_paths() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    let output = stdout(
        fixture
            .command()
            .args(["config", "paths", "--json"])
            .output()
            .unwrap(),
    );

    // Assert
    assert_eq!(
        serde_json::from_str::<Value>(&output).unwrap(),
        json!({"directory": fixture.path(),
        "workspaces":fixture.path().join("workspaces.toml"), "credentials":fixture.path().join("credentials.toml")})
    );
}

#[test]
fn test_explicit_output_json_is_supported() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    let output = stdout(
        fixture
            .command()
            .args(["config", "paths", "--output", "json"])
            .output()
            .unwrap(),
    );

    // Assert
    assert!(serde_json::from_str::<Value>(&output).is_ok());
}

#[test]
fn test_config_directory_flag_overrides_environment() {
    // Arrange
    let fixture = Fixture::new();
    let override_path = fixture.directory.path().join("override");

    // Act
    let output = stdout(
        fixture
            .command()
            .args(["config", "paths", "--json", "--config-dir"])
            .arg(&override_path)
            .output()
            .unwrap(),
    );

    // Assert
    assert_eq!(
        serde_json::from_str::<Value>(&output)
            .unwrap()
            .get("directory"),
        Some(&json!(override_path))
    );
    assert!(!override_path.exists());
}

#[test]
fn test_config_show_reads_metadata_without_loading_credentials() {
    // Arrange
    let fixture = Fixture::new();
    fixture.saved_workspace();

    // Act
    let output = stdout(
        fixture
            .command()
            .args(["config", "show", "--json"])
            .output()
            .unwrap(),
    );

    // Assert
    assert_eq!(
        serde_json::from_str::<Value>(&output)
            .unwrap()
            .get("credential_source"),
        Some(&json!("file"))
    );
    assert!(!output.contains("fk_secret"));
    assert!(!fixture.path().join("workspaces.lock").exists());
}

#[test]
fn test_config_show_does_not_create_missing_files() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    let output = stdout(fixture.command().args(["config", "show"]).output().unwrap());

    // Assert
    assert!(output.contains("No workspace selected"));
    assert!(!fixture.path().exists());
}

#[test]
fn test_config_show_reports_environment_source_without_exposing_token() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    let output = stdout(
        fixture
            .command()
            .env("FOMKEE_API_TOKEN", "fk_secret_environment")
            .args(["config", "show", "--json"])
            .output()
            .unwrap(),
    );

    // Assert
    assert!(output.contains("environment"));
    assert!(!output.contains("fk_secret"));
    assert!(!fixture.path().exists());
}

#[test]
fn test_completion_includes_saved_aliases_without_reading_secrets() {
    // Arrange
    let fixture = Fixture::new();
    fixture.saved_workspace();

    // Act
    let output = stdout(
        fixture
            .command()
            .args(["completion", "bash"])
            .output()
            .unwrap(),
    );

    // Assert
    assert!(output.contains("personal"));
    assert!(output.contains("--interval"));
    assert!(!output.contains("fk_secret"));
    assert!(!fixture.path().join("workspaces.lock").exists());
}

fn assert_completion(shell: &str) {
    let fixture = Fixture::new();
    let output = stdout(
        fixture
            .command()
            .args(["completion", shell])
            .output()
            .unwrap(),
    );
    assert!(output.contains("fomkeecli"));
    assert!(output.contains("workspace"));
    assert!(!fixture.path().exists());
}

#[test]
fn test_generates_bash_completion() {
    assert_completion("bash");
}
#[test]
fn test_generates_zsh_completion() {
    assert_completion("zsh");
}
#[test]
fn test_generates_fish_completion() {
    assert_completion("fish");
}
#[test]
fn test_generates_powershell_completion() {
    assert_completion("powershell");
}
#[test]
fn test_generates_elvish_completion() {
    assert_completion("elvish");
}

#[test]
fn test_removed_plural_command_is_rejected() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    let output = fixture
        .command()
        .args(["monitors", "list", "--json"])
        .output()
        .unwrap();

    // Assert
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stderr)
            .unwrap()
            .get("category"),
        Some(&json!("validation"))
    );
}

#[test]
fn test_missing_url_is_an_actionable_json_error_without_network_access() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    let output = fixture
        .command()
        .args(["monitor", "create", "http", "--json"])
        .output()
        .unwrap();

    // Assert
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("<URL>"));
    assert!(!fixture.path().exists());
}

#[test]
fn test_noninteractive_human_delete_requires_yes_before_authentication() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    let output = fixture
        .command()
        .args(["monitor", "delete", "00000000-0000-0000-0000-000000000001"])
        .output()
        .unwrap();

    // Assert
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--yes"));
    assert!(!fixture.path().exists());
}
