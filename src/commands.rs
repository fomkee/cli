use crate::local_commands::ConfigCommand;
use crate::model::MonitorId;
use crate::monitoring::CreateCommand;
use crate::output::{ColorMode, OutputMode};
use crate::workspace::{WorkspaceCommand, model::WorkspaceAlias};
use clap::{Args, Parser, Subcommand, ValueHint};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "fomkeecli",
    version,
    styles = crate::output::help_styles(),
    about = "Manage Fomkee monitors through the public API"
)]
pub struct Cli {
    /// Output format (human by default, even when piped). JSON never prompts.
    #[arg(long, global = true, conflicts_with = "json")]
    pub(crate) output: Option<OutputMode>,
    /// Emit JSON for automation; never prompt.
    #[arg(long, global = true)]
    pub(crate) json: bool,
    /// Color policy for human output; JSON is always unstyled.
    #[arg(long, global = true, value_enum, default_value = "auto")]
    pub(crate) color: ColorMode,
    /// Include secondary settings and audit metadata in human output.
    #[arg(long, global = true)]
    pub(crate) details: bool,
    /// Configuration directory (overrides FOMKEE_CONFIG_DIR and platform default).
    #[arg(long, global = true, value_hint = ValueHint::DirPath)]
    pub(crate) config_dir: Option<PathBuf>,
    /// Saved workspace alias. Defaults to FOMKEE_PROFILE, then environment token or active workspace.
    #[arg(long, global = true, env = "FOMKEE_PROFILE")]
    pub(crate) workspace: Option<WorkspaceAlias>,
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Export bundled agent skills for review and native import.
    Skill {
        #[command(subcommand)]
        command: SkillCommand,
    },
    /// Inspect local configuration without reading secrets.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Print a shell completion script; does not modify your shell configuration.
    #[command(
        after_help = "Bash: source <(fomkeecli completion bash)\nZsh: source <(fomkeecli completion zsh) (after compinit)\nFish: fomkeecli completion fish | source\nPowerShell: fomkeecli completion powershell | Out-String | Invoke-Expression\nAdd the corresponding command to your shell profile to enable it for future sessions."
    )]
    Completion { shell: Shell },
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    Entitlement {
        #[command(subcommand)]
        command: EntitlementsCommand,
    },
    Monitor {
        #[command(subcommand)]
        command: MonitorCommand,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum SkillCommand {
    /// Export both skills offline; never installs into an agent or overwrites a destination.
    Export {
        /// New output directory; its parent must already exist.
        #[arg(value_hint = ValueHint::DirPath)]
        directory: PathBuf,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum AuthCommand {
    Status,
}
#[derive(Debug, Subcommand)]
pub(crate) enum EntitlementsCommand {
    Show,
}
#[derive(Debug, Subcommand)]
pub(crate) enum MonitorCommand {
    List {
        #[arg(long, default_value_t = 100)]
        limit: u32,
        /// Opaque cursor returned as next_cursor by the previous list command.
        #[arg(long)]
        after: Option<String>,
    },
    Get {
        monitor_id: MonitorId,
    },
    DryRun(InputFile),
    Create {
        #[command(subcommand)]
        command: CreateCommand,
    },
    Pause {
        monitor_id: MonitorId,
        #[arg(long)]
        reason: Option<String>,
        #[arg(long)]
        resume_at: Option<String>,
    },
    Resume {
        monitor_id: MonitorId,
    },
    Disable {
        monitor_id: MonitorId,
        #[arg(long)]
        reason: Option<String>,
    },
    Enable {
        monitor_id: MonitorId,
    },
    Delete {
        monitor_id: MonitorId,
        #[arg(long)]
        yes: bool,
    },
}
#[derive(Debug, Args)]
pub(crate) struct InputFile {
    #[arg(long, default_value = "-")]
    pub(crate) file: String,
}
