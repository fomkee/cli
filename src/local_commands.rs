use std::env;
use std::io::{self, Write};
use std::path::Path;

use crate::result::CommandResult;
use crate::workspace::output::{ConfigInfo, ConfigPaths, CredentialSource};
use clap::{CommandFactory, Subcommand, builder::PossibleValuesParser};
use clap_complete::{Shell, generate};

use crate::Cli;
use crate::config::{DEFAULT_API_URL, api_url};
use crate::error::CliError;
use crate::workspace::model::WorkspaceAlias;
use crate::workspace::service;
use crate::workspace::store::{config_directory, read_registry};

/// Local, non-secret configuration diagnostics.
#[derive(Debug, Subcommand)]
pub(crate) enum ConfigCommand {
    /// Show resolved file locations, even before configuration exists.
    Paths,
    /// Show the effective connection without reading tokens or contacting the API.
    Show,
}

pub(crate) fn config(
    command: ConfigCommand,
    directory: &Path,
    alias: Option<&WorkspaceAlias>,
) -> Result<CommandResult, CliError> {
    match command {
        ConfigCommand::Paths => Ok(CommandResult::ConfigPaths(ConfigPaths {
            directory: directory.to_owned(),
            workspaces: directory.join("workspaces.toml"),
            credentials: directory.join("credentials.toml"),
        })),
        ConfigCommand::Show => show(directory, alias).map(CommandResult::ConfigShow),
    }
}

fn show(directory: &Path, alias: Option<&WorkspaceAlias>) -> Result<ConfigInfo, CliError> {
    if alias.is_none() && env::var_os("FOMKEE_API_TOKEN").is_some() {
        let origin = env::var("FOMKEE_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.into());
        return Ok(ConfigInfo::Environment {
            directory: directory.to_owned(),
            api_url: api_url(&origin)?.to_string(),
            credential_source: CredentialSource::Environment,
            alias: None,
        });
    }
    let registry = read_registry(directory)?;
    match service::select_profile(&registry, alias) {
        Ok((alias, profile)) => Ok(ConfigInfo::Saved {
            directory: directory.to_owned(),
            alias,
            api_url: api_url(&profile.api_url)?.to_string(),
            workspace_id: profile.workspace_id,
            name: profile.name,
            credential_source: profile.credential_store,
        }),
        Err(CliError::MissingCredentials) => Ok(ConfigInfo::Unselected {
            directory: directory.to_owned(),
            connection: "No workspace selected. Run fomkeecli workspace connect ALIAS.",
        }),
        Err(error) => Err(error),
    }
}

pub fn completion(shell: Shell, directory: Option<&Path>) -> Result<(), CliError> {
    let mut command = Cli::command();
    if let Ok(registry) =
        config_directory(directory).and_then(|directory| read_registry(&directory))
    {
        let aliases: Vec<String> = registry
            .workspaces
            .keys()
            .map(ToString::to_string)
            .collect();
        if !aliases.is_empty() {
            command = command.mut_arg("workspace", |arg| {
                arg.value_parser(PossibleValuesParser::new(aliases))
            });
        }
    }
    let mut script = Vec::new();
    generate(shell, &mut command, "fomkeecli", &mut script);
    io::stdout()
        .lock()
        .write_all(&script)
        .map_err(|error| CliError::io("cannot write shell completion script", error))
}
