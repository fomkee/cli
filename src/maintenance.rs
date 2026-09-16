use crate::client::MaintenanceApi;
use crate::commands::PageArgs;
use crate::dto::maintenance::{CreateMaintenanceInput, timestamp_input};
use crate::error::CliError;
use crate::model::{MaintenanceId, MonitorId, WorkspaceId};
use crate::monitoring::read_json_input;
use crate::result::CommandResult;
use clap::{Args, Subcommand};
use serde_json::json;

#[derive(Debug, Subcommand)]
pub(crate) enum MaintenanceCommand {
    /// List maintenance windows and their schedules.
    List(PageArgs),
    /// Schedule maintenance for selected monitors.
    #[command(
        after_help = "Use an explicit timezone for both times. Checks, incidents, and alerts continue normally during maintenance."
    )]
    Create(CreateArgs),
    /// Cancel a scheduled or active maintenance window.
    Cancel {
        /// Maintenance ID shown by maintenance list.
        maintenance_id: MaintenanceId,
    },
}

#[derive(Debug, Args)]
pub(crate) struct CreateArgs {
    /// Complete JSON settings file; use - for standard input.
    #[arg(long, conflicts_with_all = ["title", "description", "start", "end", "monitors"])]
    file: Option<String>,
    /// Short public title for the maintenance.
    #[arg(long, required_unless_present = "file")]
    title: Option<String>,
    /// Public description of the planned work.
    #[arg(long)]
    description: Option<String>,
    /// Start time with timezone, e.g. 2026-10-01T09:00:00Z.
    #[arg(long, required_unless_present = "file", value_parser = timestamp_input)]
    start: Option<String>,
    /// End time with timezone, e.g. 2026-10-01T10:00:00Z.
    #[arg(long, required_unless_present = "file", value_parser = timestamp_input)]
    end: Option<String>,
    /// Affected monitor IDs, separated by commas; repeat to add more.
    #[arg(
        long = "monitor",
        required_unless_present = "file",
        value_delimiter = ','
    )]
    monitors: Vec<MonitorId>,
}

impl CreateArgs {
    fn into_input(self) -> Result<CreateMaintenanceInput, CliError> {
        let value = match self.file {
            Some(path) => read_json_input(&path)?,
            None => json!({
                "title": self.title, "description": self.description,
                "scheduled_start": self.start, "scheduled_end": self.end,
                "monitors": self.monitors,
            }),
        };
        CreateMaintenanceInput::try_from(value)
    }
}

pub(crate) async fn execute(
    api: &impl MaintenanceApi,
    workspace: &WorkspaceId,
    command: MaintenanceCommand,
) -> Result<CommandResult, CliError> {
    match command {
        MaintenanceCommand::List(page) => api
            .list_maintenance(workspace, page.limit, page.after.as_deref())
            .await
            .map(CommandResult::MaintenanceList),
        MaintenanceCommand::Create(args) => api
            .create_maintenance(workspace, args.into_input()?)
            .await
            .map(CommandResult::MaintenanceCreated),
        MaintenanceCommand::Cancel { maintenance_id } => api
            .cancel_maintenance(workspace, &maintenance_id)
            .await
            .map(CommandResult::MaintenanceCancelled),
    }
}

#[cfg(test)]
mod tests;
