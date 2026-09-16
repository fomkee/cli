use crate::alerting::DestinationCommand;
use crate::incidents::IncidentCommand;
use crate::local_commands::ConfigCommand;
use crate::maintenance::MaintenanceCommand;
use crate::model::DestinationId;
use crate::model::MonitorId;
use crate::monitoring::CreateCommand;
use crate::monitoring::update::UpdateArgs;
use crate::output::{ColorMode, OutputMode};
use crate::workspace::{WorkspaceCommand, model::WorkspaceAlias};
use clap::builder::BoolishValueParser;
use clap::{Args, Parser, Subcommand, ValueHint};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "fomkeecli",
    version,
    styles = crate::output::help_styles(),
    about = "Manage Fomkee monitors, alerts, and workspaces"
)]
pub struct Cli {
    /// Choose human-readable or JSON output.
    #[arg(long, global = true, conflicts_with = "json")]
    pub(crate) output: Option<OutputMode>,
    /// Print JSON for scripts and automation.
    #[arg(long, global = true)]
    pub(crate) json: bool,
    /// Choose when to use colored output.
    #[arg(long, global = true, value_enum, default_value = "auto")]
    pub(crate) color: ColorMode,
    /// Show additional settings and history.
    #[arg(long, global = true)]
    pub(crate) details: bool,
    /// Skip new-version notifications.
    #[arg(long, global = true, env = "FOMKEE_NO_UPDATE_CHECK", value_parser = BoolishValueParser::new())]
    pub(crate) no_update_check: bool,
    /// Use a different configuration directory.
    #[arg(long, global = true, value_hint = ValueHint::DirPath)]
    pub(crate) config_dir: Option<PathBuf>,
    /// Use a saved workspace by its alias.
    #[arg(long, global = true, env = "FOMKEE_PROFILE")]
    pub(crate) workspace: Option<WorkspaceAlias>,
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Inspect incidents and publish public updates.
    Incident {
        #[command(subcommand)]
        command: IncidentCommand,
    },
    /// Schedule and manage planned maintenance.
    Maintenance {
        #[command(subcommand)]
        command: MaintenanceCommand,
    },
    /// Update fomkeecli to the latest stable version.
    SelfUpdate {
        /// Check for a new version without installing it.
        #[arg(long)]
        check: bool,
    },
    /// Export instructions for AI coding assistants.
    Skill {
        #[command(subcommand)]
        command: SkillCommand,
    },
    /// View your CLI settings and file locations.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Generate tab completion for your shell.
    #[command(
        after_help = "Bash: source <(fomkeecli completion bash)\nZsh: source <(fomkeecli completion zsh) (after compinit)\nFish: fomkeecli completion fish | source\nPowerShell: fomkeecli completion powershell | Out-String | Invoke-Expression\nAdd the corresponding command to your shell profile to enable it for future sessions."
    )]
    Completion {
        /// Shell to generate completions for.
        shell: Shell,
    },
    /// Connect, select, and manage saved workspaces.
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
    /// Check your current connection and access.
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    /// View your plan features and usage limits.
    Entitlement {
        #[command(subcommand)]
        command: EntitlementsCommand,
    },
    /// Manage where monitor alerts are sent.
    Destination {
        #[command(subcommand)]
        command: DestinationCommand,
    },
    /// Create, inspect, and manage monitors.
    Monitor {
        #[command(subcommand)]
        command: MonitorCommand,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum SkillCommand {
    /// Save the bundled AI skills to a new folder.
    #[command(
        after_help = "Works offline. The destination must be new; its parent must already exist."
    )]
    Export {
        /// New folder to create inside an existing directory.
        #[arg(value_hint = ValueHint::DirPath)]
        directory: PathBuf,
    },
}
#[derive(Debug, Subcommand)]
pub(crate) enum AuthCommand {
    /// Show your current workspace and access.
    Status,
}
#[derive(Debug, Subcommand)]
pub(crate) enum EntitlementsCommand {
    /// Show your plan features and usage limits.
    Show,
}
#[derive(Debug, Subcommand)]
pub(crate) enum MonitorCommand {
    /// List monitors in the selected workspace.
    List {
        /// Maximum results to show, from 1 to 100.
        #[arg(long, default_value_t = 100)]
        limit: u32,
        /// Continue from a previous result's next_cursor.
        #[arg(long)]
        after: Option<String>,
    },
    /// Show a monitor's status and settings.
    Get {
        /// Monitor ID shown by monitor list.
        monitor_id: MonitorId,
    },
    /// Change a monitor's settings.
    Update {
        /// Monitor ID shown by monitor list.
        monitor_id: MonitorId,
        #[command(flatten)]
        changes: Box<UpdateArgs>,
    },
    /// List destinations assigned to this monitor.
    Destinations {
        /// Monitor ID shown by monitor list.
        monitor_id: MonitorId,
        /// Maximum results to show, from 1 to 100.
        #[arg(long, default_value_t = 100)]
        limit: u32,
        /// Continue from a previous result's next_cursor.
        #[arg(long)]
        after: Option<String>,
    },
    /// Send this monitor's alerts to a destination.
    Assign {
        /// Monitor ID shown by monitor list.
        monitor_id: MonitorId,
        /// Destination ID shown by destination list.
        destination_id: DestinationId,
    },
    /// Try an HTTP or Function check without saving a monitor.
    DryRun(InputFile),
    /// Create an HTTP, Function, or Heartbeat monitor.
    Create {
        #[command(subcommand)]
        command: CreateCommand,
    },
    /// Temporarily pause monitoring.
    Pause {
        /// Monitor ID shown by monitor list.
        monitor_id: MonitorId,
        /// Reason for stopping monitoring.
        #[arg(long)]
        reason: Option<String>,
        /// Resume at this time, e.g. 2026-10-01T09:00:00Z.
        #[arg(long)]
        resume_at: Option<String>,
    },
    /// Resume a paused monitor.
    Resume {
        /// Monitor ID shown by monitor list.
        monitor_id: MonitorId,
    },
    /// Turn off a monitor until you enable it again.
    Disable {
        /// Monitor ID shown by monitor list.
        monitor_id: MonitorId,
        /// Reason for stopping monitoring.
        #[arg(long)]
        reason: Option<String>,
    },
    /// Turn on a disabled monitor.
    Enable {
        /// Monitor ID shown by monitor list.
        monitor_id: MonitorId,
    },
    /// Permanently delete a monitor.
    Delete {
        /// Monitor ID shown by monitor list.
        monitor_id: MonitorId,
        /// Confirm deletion without a prompt.
        #[arg(long)]
        yes: bool,
    },
}
#[derive(Debug, Args)]
pub(crate) struct InputFile {
    /// JSON settings file; use - to read from standard input.
    #[arg(long, default_value = "-")]
    pub(crate) file: String,
}

#[derive(Debug, Args)]
pub(crate) struct PageArgs {
    /// Maximum results to show, from 1 to 100.
    #[arg(long, default_value_t = 100)]
    pub limit: u32,
    /// Continue from a previous result's next_cursor.
    #[arg(long)]
    pub after: Option<String>,
}
