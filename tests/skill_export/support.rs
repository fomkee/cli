use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};

pub(super) struct Fixture {
    pub(super) root: TempDir,
}

impl Fixture {
    pub(super) fn new() -> Self {
        Self {
            root: tempdir().unwrap(),
        }
    }

    pub(super) fn destination(&self) -> PathBuf {
        self.root.path().join("review skills")
    }

    pub(super) fn config(&self) -> PathBuf {
        self.root.path().join("config")
    }

    pub(super) fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_fomkeecli"));
        command
            .current_dir(self.root.path())
            .env("FOMKEE_CONFIG_DIR", self.config())
            .env_remove("FOMKEE_API_TOKEN")
            .env_remove("FOMKEE_API_URL")
            .env_remove("FOMKEE_PROFILE")
            .env("NO_COLOR", "1");
        command
    }

    pub(super) fn export(&self) -> Command {
        let mut command = self.command();
        command.args(["skill", "export"]).arg(self.destination());
        command
    }
}

pub(super) fn stdout(command: &mut Command) -> String {
    let output = command.output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    String::from_utf8(output.stdout).unwrap()
}

pub(super) fn failure(command: &mut Command, code: i32) -> String {
    let output = command.output().unwrap();
    assert_eq!(output.status.code(), Some(code), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    String::from_utf8(output.stderr).unwrap()
}

pub(super) fn files(directory: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    collect_files(directory, directory)
}

fn collect_files(root: &Path, directory: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fs::read_dir(directory)
        .unwrap()
        .flat_map(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect_files(root, &path)
            } else {
                BTreeMap::from([(
                    path.strip_prefix(root).unwrap().to_owned(),
                    fs::read(&path).unwrap(),
                )])
            }
        })
        .collect()
}

pub(super) fn assert_matches_sources(directory: &Path) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("skills");
    assert_eq!(files(directory), files(&source));
}

pub(super) fn assert_manifest_matches_export(output: &str, directory: &Path) {
    let value: Value = serde_json::from_str(output).unwrap();
    assert_eq!(value.get("directory"), Some(&json!(directory)));
    assert_eq!(value.get("cli_version").unwrap(), env!("CARGO_PKG_VERSION"));
    let exported: BTreeMap<PathBuf, Vec<u8>> = value
        .get("skills")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|skill| {
            let name = skill.get("name").unwrap().as_str().unwrap();
            assert_eq!(
                skill.get("directory").unwrap(),
                &json!(directory.join(name))
            );
            skill
                .get("files")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(move |file| {
                    let relative = Path::new(name).join(file.as_str().unwrap());
                    let content = fs::read(directory.join(&relative)).unwrap();
                    (relative, content)
                })
        })
        .collect();
    assert_eq!(exported, files(directory));
}

pub(super) fn assert_absolute_manifest_matches_export(output: &str, directory: &Path) {
    let value: Value = serde_json::from_str(output).unwrap();
    let reported = Path::new(value.get("directory").unwrap().as_str().unwrap());
    assert!(reported.is_absolute());
    assert_eq!(
        reported.canonicalize().unwrap(),
        directory.canonicalize().unwrap()
    );
    assert_manifest_matches_export(output, reported);
}
