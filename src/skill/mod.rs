use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, absolute};

use serde::Serialize;
use std::path::PathBuf;

use crate::error::CliError;

mod bundled;
use bundled::SKILLS;

pub(crate) trait ExportSkills {
    fn export(&self, directory: &Path) -> Result<(), CliError>;
}

pub(crate) struct FileSkillExporter;

impl ExportSkills for FileSkillExporter {
    fn export(&self, directory: &Path) -> Result<(), CliError> {
        fs::create_dir(directory).map_err(|error| destination_error(directory, error))?;
        write_bundle(directory).map_err(|error| {
            CliError::SkillExport(format!(
                "export incomplete at {}: {error}; review the partial directory and retry with a new destination",
                directory.display()
            ))
        })
    }
}

pub(crate) fn export(
    exporter: &impl ExportSkills,
    directory: &Path,
) -> Result<ExportedSkills, CliError> {
    let directory = absolute(directory).map_err(|error| {
        CliError::SkillExport(format!("cannot resolve the destination: {error}"))
    })?;
    exporter.export(&directory)?;
    let skills = SKILLS
        .iter()
        .map(|skill| ExportedSkill {
            name: skill.name,
            directory: directory.join(skill.name),
            files: skill.files.iter().map(|(path, _)| *path).collect(),
        })
        .collect();
    Ok(ExportedSkills {
        directory,
        cli_version: env!("CARGO_PKG_VERSION"),
        skills,
    })
}

#[derive(Serialize)]
pub(crate) struct ExportedSkills {
    pub(crate) directory: PathBuf,
    pub(crate) cli_version: &'static str,
    pub(crate) skills: Vec<ExportedSkill>,
}

#[derive(Serialize)]
pub(crate) struct ExportedSkill {
    pub(crate) name: &'static str,
    pub(crate) directory: PathBuf,
    pub(crate) files: Vec<&'static str>,
}

fn destination_error(directory: &Path, error: io::Error) -> CliError {
    if error.kind() == io::ErrorKind::AlreadyExists {
        return CliError::InvalidInput(format!(
            "destination {} already exists; choose a new directory. Nothing was overwritten",
            directory.display()
        ));
    }
    CliError::SkillExport(format!(
        "cannot create {}: {error}; choose a writable destination with an existing parent directory",
        directory.display()
    ))
}

fn write_bundle(directory: &Path) -> io::Result<()> {
    for skill in SKILLS {
        let root = directory.join(skill.name);
        fs::create_dir(&root)?;
        for (relative, contents) in skill.files {
            let path = root.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
            file.write_all(contents)?;
        }
    }
    Ok(())
}
