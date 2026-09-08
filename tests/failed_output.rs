#![cfg(test)]
#![cfg(target_os = "linux")]

use std::fs::OpenOptions;
use std::process::{Command, Output};

fn failed_stdout(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fomkeecli"))
        .args(arguments)
        .env_remove("FOMKEE_API_TOKEN")
        .stdout(OpenOptions::new().write(true).open("/dev/full").unwrap())
        .output()
        .unwrap()
}

fn assert_output_failure(output: &Output) {
    assert_eq!(output.status.code(), Some(7));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot write"));
}

#[test]
fn test_help_reports_failed_stdout_instead_of_success() {
    // Arrange
    let arguments = ["--help"];
    // Act
    let output = failed_stdout(&arguments);
    // Assert
    assert_output_failure(&output);
}

#[test]
fn test_version_reports_failed_stdout_instead_of_success() {
    // Arrange
    let arguments = ["--version"];
    // Act
    let output = failed_stdout(&arguments);
    // Assert
    assert_output_failure(&output);
}

#[test]
fn test_completion_reports_failed_stdout_instead_of_success() {
    // Arrange
    let arguments = ["completion", "bash"];
    // Act
    let output = failed_stdout(&arguments);
    // Assert
    assert_output_failure(&output);
}
