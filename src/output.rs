use crate::error::CliError;
use crate::error::transport::TransportFailure;
use crate::result::CommandResult;
use anstream::{AutoStream, ColorChoice};
use clap::{ValueEnum, builder::Styles};
use std::env;
use std::io::IsTerminal;
use std::io::{self, Write};
use terminal_size::{Width, terminal_size};

mod connection;
mod context;
mod dry_run;
mod format;
mod human;
mod incidents;
mod layout;
mod maintenance;
mod monitor;
mod skill;
mod theme;
pub(crate) use context::DisplayContext;

pub(crate) fn help_styles() -> Styles {
    Styles::styled()
        .header(theme::SECTION)
        .usage(theme::SECTION)
        .literal(theme::TITLE)
        .error(theme::FAILURE)
        .valid(theme::SUCCESS)
        .invalid(theme::WARNING)
}

/// Color preference for human output; machine output is always unstyled.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

impl ColorMode {
    pub(crate) fn choice(self, terminal: bool) -> ColorChoice {
        match self {
            Self::Always => ColorChoice::Always,
            Self::Never => ColorChoice::Never,
            Self::Auto
                if !terminal
                    || env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty())
                    || env::var_os("TERM").is_some_and(|value| value == "dumb") =>
            {
                ColorChoice::Never
            }
            Self::Auto => ColorChoice::Auto,
        }
    }
}

pub(crate) struct Presentation {
    pub(crate) mode: OutputMode,
    pub(crate) color: ColorMode,
    pub(crate) details: bool,
    pub(crate) context: DisplayContext,
}

fn width(terminal: bool) -> u16 {
    if terminal {
        terminal_size()
            .map(|(Width(width), _)| width)
            .unwrap_or(100)
    } else {
        100
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputMode {
    Json,
    Human,
}
pub struct Output;
impl Output {
    pub(crate) fn write(value: CommandResult, options: &Presentation) -> Result<(), CliError> {
        let stdout = io::stdout();
        let terminal = stdout.is_terminal();
        let mut stream = AutoStream::new(stdout.lock(), options.color.choice(terminal));
        match options.mode {
            OutputMode::Json => {
                serde_json::to_writer(&mut stream, &value)
                    .map_err(|error| CliError::Transport(TransportFailure::JsonOutput(error)))?;
                writeln!(stream)
            }
            OutputMode::Human => writeln!(
                stream,
                "{}",
                human::render(
                    &value,
                    options.details,
                    width(terminal),
                    stream.current_choice() != ColorChoice::Never,
                    &options.context,
                )
            ),
        }
        .and_then(|()| stream.flush())
        .map_err(|error| CliError::io("cannot write output", error))
    }
    /// Best-effort diagnostics after failure; callers still return a nonzero exit if stderr is unavailable.
    pub(crate) fn write_error(error: &CliError, mode: OutputMode, color: ColorMode) {
        match mode {
            OutputMode::Json => {
                let _ = serde_json::to_writer(io::stderr().lock(), &error.as_json());
                let _ = writeln!(io::stderr().lock());
            }
            OutputMode::Human => {
                let stderr = io::stderr();
                let terminal = stderr.is_terminal();
                let mut stream = AutoStream::new(stderr.lock(), color.choice(terminal));
                let rendered = human::error(
                    error,
                    width(terminal),
                    stream.current_choice() != ColorChoice::Never,
                );
                let _ = writeln!(stream, "{rendered}");
            }
        }
    }
    pub fn confirm_delete(monitor_id: &str) -> Result<bool, CliError> {
        if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
            return Err(CliError::ConfirmationRequired);
        }
        let mut answer = String::new();
        write!(
            io::stderr().lock(),
            "Delete monitor {monitor_id}? Type delete to continue: "
        )
        .map_err(|error| CliError::io("cannot prompt for confirmation", error))?;
        io::stderr()
            .flush()
            .map_err(|error| CliError::io("cannot prompt for confirmation", error))?;
        io::stdin()
            .read_line(&mut answer)
            .map_err(|error| CliError::io("cannot read confirmation", error))?;
        Ok(answer.trim() == "delete")
    }
}

mod alerting;
