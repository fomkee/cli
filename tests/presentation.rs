#![cfg(test)]

use std::process::{Command, Output};

use serde_json::Value;

fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fomkeecli"));
    command
        .env_remove("FOMKEE_API_TOKEN")
        .env_remove("FOMKEE_PROFILE")
        .env_remove("FOMKEE_WORKSPACE_ID")
        .env_remove("NO_COLOR")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("CLICOLOR")
        .env("TERM", "xterm-256color");
    command
}

fn succeeded(output: Output) -> String {
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn test_redirected_output_is_human_and_plain_by_default() {
    // Arrange
    let mut command = command();
    command.args(["config", "paths"]);

    // Act
    let output = succeeded(command.output().unwrap());

    // Assert
    assert!(output.starts_with("Configuration paths"));
    assert!(!output.contains('\u{1b}'));
}

#[test]
fn test_always_color_overrides_no_color_and_dumb_terminal() {
    // Arrange
    let mut command = command();
    command
        .args(["config", "paths", "--color", "always"])
        .env("NO_COLOR", "1")
        .env("TERM", "dumb");

    // Act
    let output = succeeded(command.output().unwrap());

    // Assert
    assert!(output.contains("\u{1b}[1m") && output.contains("Configuration paths"));
}

#[test]
fn test_never_color_overrides_force_color_environment() {
    // Arrange
    let mut command = command();
    command
        .args(["config", "paths", "--color", "never"])
        .env("CLICOLOR_FORCE", "1");

    // Act
    let output = succeeded(command.output().unwrap());

    // Assert
    assert!(!output.contains('\u{1b}'));
}

#[test]
fn test_json_is_unchanged_by_details_and_forced_color() {
    // Arrange
    let baseline = succeeded(
        command()
            .args(["config", "paths", "--json"])
            .output()
            .unwrap(),
    );

    // Act
    let output = succeeded(
        command()
            .args(["config", "paths", "--json", "--details", "--color=always"])
            .output()
            .unwrap(),
    );

    // Assert
    assert_eq!(output, baseline);
    assert!(serde_json::from_str::<Value>(&output).is_ok());
}

#[test]
fn test_human_errors_are_colored_on_stderr_only_when_requested() {
    // Arrange
    let mut command = command();
    command.args([
        "monitor",
        "delete",
        "00000000-0000-0000-0000-000000000001",
        "--color=always",
    ]);

    // Act
    let output = command.output().unwrap();

    // Assert
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr).unwrap().contains("[31m"));
}

#[test]
fn test_json_argument_errors_never_contain_ansi_with_forced_color() {
    // Arrange
    let mut command = command();
    command.args(["--json", "--color=always", "unknown-command"]);

    // Act
    let output = command.output().unwrap();

    // Assert
    assert_eq!(output.status.code(), Some(2));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert!(!error.to_string().contains("\\u001b"));
}

#[test]
fn test_help_respects_explicit_color_preference() {
    // Arrange
    let mut command = command();
    command
        .args(["--color=always", "--help"])
        .env("NO_COLOR", "1");

    // Act
    let output = succeeded(command.output().unwrap());

    // Assert
    assert!(output.contains('\u{1b}') && output.contains("--details"));
}

#[test]
fn test_help_remains_plain_when_piped_by_default() {
    // Arrange
    let mut command = command();
    command.args(["monitor", "--help"]);

    // Act
    let output = succeeded(command.output().unwrap());

    // Assert
    assert!(output.contains("Usage:") && !output.contains('\u{1b}'));
}

#[test]
fn test_completion_is_native_shell_text_even_with_presentation_flags() {
    // Arrange
    let mut command = command();
    command.args([
        "completion",
        "bash",
        "--details",
        "--color=always",
        "--json",
    ]);

    // Act
    let output = succeeded(command.output().unwrap());

    // Assert
    assert!(output.contains("_fomkeecli") && output.contains("--color"));
    assert!(!output.contains('\u{1b}'));
}
