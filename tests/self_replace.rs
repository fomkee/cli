#![cfg(test)]
use fomkee_cli::update::{ExecutableInstaller, RunningExecutable};
use std::env;
use std::fs;
use std::process::Command;
const CHILD: &str = "FOMKEE_TEST_REPLACE_EXECUTABLE";
const NEW_BYTES: &[u8] = b"verified replacement fixture";

#[test]
fn test_replaces_disposable_executable_copy() {
    // Arrange
    let directory = tempfile::tempdir().unwrap();
    let executable = directory
        .path()
        .join(format!("replace-fixture{}", env::consts::EXE_SUFFIX));
    fs::copy(env::current_exe().unwrap(), &executable).unwrap();
    let permissions = fs::metadata(&executable).unwrap().permissions();
    // Act
    let output = Command::new(&executable)
        .args([
            "--exact",
            "test_child_installs_only_when_requested",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .output()
        .unwrap();
    // Assert
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        fs::metadata(&executable).unwrap().permissions(),
        permissions
    );
    assert_eq!(fs::read(executable).unwrap(), NEW_BYTES);
}
#[test]
fn test_child_installs_only_when_requested() {
    install_in_child();
}
fn install_in_child() {
    if env::var_os(CHILD).is_some() {
        RunningExecutable.install(NEW_BYTES).unwrap();
    }
}
