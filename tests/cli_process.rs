use std::error::Error;
use std::process::Command;

use serde_json::Value;
use tempfile::tempdir;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_fomkeecli")
}

#[test]
fn test_missing_credentials_use_stderr_json_and_exit_three() -> Result<(), Box<dyn Error>> {
    // Arrange
    let mut command = Command::new(binary());
    command.arg("--json");
    let directory = tempdir()?;
    command.env("FOMKEE_CONFIG_DIR", directory.path());
    command.env_remove("FOMKEE_PROFILE");
    command.env_remove("FOMKEE_WORKSPACE_ID");
    command.args(["auth", "status"]);
    command.env_remove("FOMKEE_API_TOKEN");

    // Act
    let output = command.output()?;

    // Assert
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr)?;
    assert_eq!(
        error.get("category"),
        Some(&Value::String("credentials".into()))
    );
    assert!(
        error
            .get("message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("FOMKEE_API_TOKEN"))
    );
    Ok(())
}

#[test]
fn test_json_delete_without_yes_fails_before_network_access() -> Result<(), Box<dyn Error>> {
    // Arrange
    let mut command = Command::new(binary());
    command.arg("--json");
    command.args(["monitor", "delete", "00000000-0000-0000-0000-000000000002"]);
    command.env("FOMKEE_API_TOKEN", "fk_process-test-secret");
    command.env("FOMKEE_API_URL", "http://127.0.0.1:1/");

    // Act
    let output = command.output()?;

    // Assert
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(!stderr.contains("process-test-secret"));
    let error: Value = serde_json::from_str(&stderr)?;
    assert_eq!(
        error.get("category"),
        Some(&Value::String("confirmation_required".into()))
    );
    assert!(
        error
            .get("message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("confirmation required"))
    );
    Ok(())
}

#[test]
fn test_failed_identity_check_stops_before_mutation_without_secret() -> Result<(), Box<dyn Error>> {
    // Arrange
    let mut command = Command::new(binary());
    command.arg("--json");
    command.env_remove("FOMKEE_PROFILE");
    command.env_remove("FOMKEE_WORKSPACE_ID");
    command.args([
        "monitor",
        "delete",
        "00000000-0000-0000-0000-000000000002",
        "--yes",
    ]);
    command.env("FOMKEE_API_TOKEN", "fk_ambiguous-test-secret");
    command.env("FOMKEE_API_URL", "http://127.0.0.1:1/");

    // Act
    let output = command.output()?;

    // Assert
    assert_eq!(output.status.code(), Some(7));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(!stderr.contains("ambiguous-test-secret"));
    let error: Value = serde_json::from_str(&stderr)?;
    assert_eq!(
        error.get("category"),
        Some(&Value::String("transport".into()))
    );
    Ok(())
}
