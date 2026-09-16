#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo)]

pub mod alerting;
pub mod client;
mod commands;
pub mod config;
pub mod dto;
mod duration;
pub mod error;
mod local_commands;
pub mod model;
pub mod monitoring;
mod output;
mod result;
mod skill;
pub mod update;
pub mod wire;
pub mod workspace;

use std::env;
use std::ffi::OsString;
use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use crate::monitoring::execute::monitor_command;
use crate::result::CommandResult;
use anstream::AutoStream;
use clap::{CommandFactory, FromArgMatches, error::ErrorKind};

use crate::client::{FomkeeApi, HttpFomkeeApi};
pub(crate) use crate::commands::Cli;
use crate::commands::{AuthCommand, Command, EntitlementsCommand, MonitorCommand, SkillCommand};
use crate::error::CliError;
use crate::output::{ColorMode, DisplayContext, Output, OutputMode, Presentation};
use crate::workspace::store::config_directory;

/// Execute one CLI invocation, reporting errors through a controlled exit code.
pub async fn run_process() -> ExitCode {
    let mut cli = match parse_cli() {
        Ok(cli) => cli,
        Err(exit) => return exit,
    };
    if cli.json {
        cli.output = Some(OutputMode::Json);
    }
    let output_mode = cli.output.unwrap_or(OutputMode::Human);
    let mut presentation = Presentation {
        mode: output_mode,
        color: cli.color,
        details: cli.details,
        context: DisplayContext::default(),
    };
    if let Command::Completion { shell } = cli.command {
        return match local_commands::completion(shell, cli.config_dir.as_deref()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                Output::write_error(&error, output_mode, presentation.color);
                ExitCode::from(error.exit_code())
            }
        };
    }
    let update_directory = (!cli.no_update_check
        && matches!(output_mode, OutputMode::Human)
        && io::stdout().is_terminal()
        && io::stderr().is_terminal()
        && matches!(
            cli.command,
            Command::Monitor { .. }
                | Command::Destination { .. }
                | Command::Entitlement { .. }
                | Command::Auth { .. }
        ))
    .then(|| config_directory(cli.config_dir.as_deref()).ok())
    .flatten();
    match execute(cli, &mut presentation)
        .await
        .and_then(|value| Output::write(value, &presentation))
    {
        Ok(()) => {
            if let Some(directory) = update_directory {
                update::notify(&directory).await;
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            Output::write_error(&error, output_mode, presentation.color);
            ExitCode::from(error.exit_code())
        }
    }
}

fn parse_cli() -> Result<Cli, ExitCode> {
    let arguments: Vec<_> = env::args_os().collect();
    let color = argument_color(&arguments);
    let command = Cli::command().color(match color {
        ColorMode::Always => clap::ColorChoice::Always,
        ColorMode::Never => clap::ColorChoice::Never,
        ColorMode::Auto => clap::ColorChoice::Auto,
    });
    match command
        .try_get_matches_from(&arguments)
        .and_then(|matches| Cli::from_arg_matches(&matches))
    {
        Ok(cli) => Ok(cli),
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            match print_argument_message(&error, color) {
                Ok(()) => Err(ExitCode::SUCCESS),
                Err(error) => {
                    Output::write_error(&error, OutputMode::Human, color);
                    Err(ExitCode::from(error.exit_code()))
                }
            }
        }
        Err(error) => {
            let json = arguments
                .iter()
                .any(|arg| arg == "--json" || arg == "--output=json")
                || arguments.windows(2).any(|pair| {
                    pair.first().is_some_and(|arg| arg == "--output")
                        && pair.get(1).is_some_and(|arg| arg == "json")
                });
            if json {
                Output::write_error(
                    &CliError::InvalidInput(error.to_string()),
                    OutputMode::Json,
                    ColorMode::Never,
                );
            } else {
                if let Err(write_error) = print_argument_message(&error, color) {
                    Output::write_error(&write_error, OutputMode::Human, color);
                }
            }
            Err(ExitCode::from(2))
        }
    }
}

fn argument_color(arguments: &[OsString]) -> ColorMode {
    let mut color = ColorMode::Auto;
    let mut arguments = arguments.iter().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--" {
            break;
        }
        let value = if argument == "--color" {
            arguments.next().and_then(|value| value.to_str())
        } else {
            argument
                .to_str()
                .and_then(|value| value.strip_prefix("--color="))
        };
        color = match value {
            Some("always") => ColorMode::Always,
            Some("never") => ColorMode::Never,
            Some("auto") => ColorMode::Auto,
            _ => color,
        };
    }
    color
}

fn print_argument_message(error: &clap::Error, color: ColorMode) -> Result<(), CliError> {
    let rendered = error.render().ansi().to_string();
    if error.use_stderr() {
        let mut stream = AutoStream::new(io::stderr(), color.choice(io::stderr().is_terminal()));
        write!(stream, "{rendered}")
            .and_then(|()| stream.flush())
            .map_err(|error| CliError::io("cannot write help or argument output", error))
    } else {
        let mut stream = AutoStream::new(io::stdout(), color.choice(io::stdout().is_terminal()));
        write!(stream, "{rendered}")
            .and_then(|()| stream.flush())
            .map_err(|error| CliError::io("cannot write help or argument output", error))
    }
}

async fn execute(cli: Cli, presentation: &mut Presentation) -> Result<CommandResult, CliError> {
    if let Command::SelfUpdate { check } = cli.command {
        return update::command(check).await.map(CommandResult::SelfUpdate);
    }
    if let Command::Skill {
        command: SkillCommand::Export { directory },
    } = cli.command
    {
        return skill::export(&skill::FileSkillExporter, &directory)
            .map(CommandResult::SkillExport);
    }
    let directory = config_directory(cli.config_dir.as_deref())?;
    if let Command::Config { command } = cli.command {
        return local_commands::config(command, &directory, cli.workspace.as_ref());
    }
    if let Command::Workspace { command } = cli.command {
        return workspace::execute(command, cli.output, directory).await;
    }
    if matches!(
        cli.command,
        Command::Monitor {
            command: MonitorCommand::Delete { yes: false, .. }
        }
    ) && (matches!(cli.output, Some(OutputMode::Json))
        || !io::stdin().is_terminal()
        || !io::stderr().is_terminal())
    {
        return Err(CliError::ConfirmationRequired);
    }
    let connection = workspace::resolve(cli.workspace.as_ref(), directory.clone())?;
    let api = HttpFomkeeApi::new(&connection.config)?;
    let session = api.session().await?;
    let workspace = connection.verify_session(&session)?;
    presentation.context = DisplayContext::from_connection(&connection, workspace);
    match cli.command {
        Command::Auth {
            command: AuthCommand::Status,
        } => Ok(CommandResult::Auth(connection.status(session))),
        Command::Entitlement {
            command: EntitlementsCommand::Show,
        } => api
            .entitlements(&workspace)
            .await
            .map(CommandResult::Entitlement),
        Command::Destination { command } => alerting::execute(&api, &workspace, command).await,
        Command::Monitor { command } => {
            monitor_command(
                &api,
                command,
                workspace,
                cli.output.unwrap_or(OutputMode::Human),
            )
            .await
        }
        Command::Workspace { command } => workspace::execute(command, cli.output, directory).await,
        Command::Config { command } => {
            local_commands::config(command, &directory, cli.workspace.as_ref())
        }
        Command::Skill {
            command: SkillCommand::Export { directory },
        } => skill::export(&skill::FileSkillExporter, &directory).map(CommandResult::SkillExport),
        Command::SelfUpdate { check } => {
            update::command(check).await.map(CommandResult::SelfUpdate)
        }
        Command::Completion { .. } => Err(CliError::InvalidInput(
            "completion must be handled locally".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::InMemoryFomkeeApi;
    #[tokio::test]
    async fn test_uses_session_workspace_when_not_selected() {
        // Arrange
        let api = InMemoryFomkeeApi::with_session(
            "00000000-0000-0000-0000-000000000001".parse().unwrap(),
        );

        // Act
        let session = api.session().await.unwrap();
        let workspace = workspace::service::session_workspace(&session).unwrap();
        let result = monitor_command(
            &api,
            MonitorCommand::List {
                limit: 100,
                after: None,
            },
            workspace,
            OutputMode::Json,
        )
        .await;

        // Assert
        assert!(result.is_ok());
        let result = match result {
            Ok(result) => result,
            Err(_) => return,
        };
        assert!(matches!(result, CommandResult::MonitorList(page) if page.data().items.is_empty()));
    }
}
