use crate::client::FomkeeApi;
use crate::commands::InputFile;
use crate::dto::alerting::{CreateDestinationInput, UpdateDestinationInput};
use crate::error::CliError;
use crate::model::{AssignmentId, DestinationId, MonitorId, WorkspaceId};
use crate::monitoring::read_json_input;
use crate::result::CommandResult;
use clap::Subcommand;
use serde::Serialize;

/// Select one explicit assignment page by monitor or destination.
#[derive(Debug, Clone, Copy)]
pub enum AssignmentScope {
    Monitor(MonitorId),
    Destination(DestinationId),
}
#[derive(Debug, Subcommand)]
pub(crate) enum DestinationCommand {
    /// List alert destinations in the selected workspace.
    List {
        /// Maximum results to show, from 1 to 100.
        #[arg(long, default_value_t = 100)]
        limit: u32,
        /// Continue from a previous result's next_cursor.
        #[arg(long)]
        after: Option<String>,
    },
    /// Show an alert destination's settings.
    Get {
        /// Destination ID shown by destination list.
        destination_id: DestinationId,
    },
    /// Create an alert destination from a JSON settings file.
    Create(InputFile),
    /// Change an alert destination's name or settings.
    Update {
        /// Destination ID shown by destination list.
        destination_id: DestinationId,
        /// New name for this destination.
        #[arg(long, required_unless_present = "file", conflicts_with = "file")]
        name: Option<String>,
        /// Complete replacement settings in JSON; use - for standard input.
        #[arg(long)]
        file: Option<String>,
    },
    /// Send a test notification to this destination.
    Test {
        /// Destination ID shown by destination list.
        destination_id: DestinationId,
    },
    /// List monitors that send alerts to this destination.
    Monitors {
        /// Destination ID shown by destination list.
        destination_id: DestinationId,
        /// Maximum results to show, from 1 to 100.
        #[arg(long, default_value_t = 100)]
        limit: u32,
        /// Continue from a previous result's next_cursor.
        #[arg(long)]
        after: Option<String>,
    },
    /// Send a monitor's alerts to this destination.
    Assign {
        /// Destination ID shown by destination list.
        destination_id: DestinationId,
        /// Monitor ID shown by monitor list.
        monitor_id: MonitorId,
    },
    /// Stop sending a monitor's alerts to a destination.
    Unassign {
        /// Assignment ID shown by destination monitors or monitor destinations.
        assignment_id: AssignmentId,
    },
}
#[derive(Serialize)]
pub(crate) struct RemovedAssignment {
    pub removed: bool,
    pub assignment_id: AssignmentId,
}

pub(crate) async fn execute(
    api: &impl FomkeeApi,
    workspace: &WorkspaceId,
    command: DestinationCommand,
) -> Result<CommandResult, CliError> {
    match command {
        DestinationCommand::List { limit, after } => api
            .list_destinations(workspace, limit, after.as_deref())
            .await
            .map(CommandResult::DestinationList),
        DestinationCommand::Get { destination_id } => api
            .get_destination(workspace, &destination_id)
            .await
            .map(CommandResult::Destination),
        DestinationCommand::Create(input) => api
            .create_destination(
                workspace,
                CreateDestinationInput::try_from(read_json_input(&input.file)?)?,
            )
            .await
            .map(CommandResult::DestinationCreated),
        DestinationCommand::Update {
            destination_id,
            name,
            file,
        } => update(api, workspace, &destination_id, name, file).await,
        DestinationCommand::Test { destination_id } => api
            .test_destination(workspace, &destination_id)
            .await
            .map(CommandResult::DestinationTest),
        DestinationCommand::Monitors {
            destination_id,
            limit,
            after,
        } => api
            .list_assignments(
                workspace,
                AssignmentScope::Destination(destination_id),
                limit,
                after.as_deref(),
            )
            .await
            .map(CommandResult::Assignments),
        DestinationCommand::Assign {
            destination_id,
            monitor_id,
        } => api
            .assign_destination(workspace, &monitor_id, &destination_id)
            .await
            .map(CommandResult::Assigned),
        DestinationCommand::Unassign { assignment_id } => {
            api.remove_assignment(workspace, &assignment_id).await?;
            Ok(CommandResult::Unassigned(RemovedAssignment {
                removed: true,
                assignment_id,
            }))
        }
    }
}
async fn update(
    api: &impl FomkeeApi,
    workspace: &WorkspaceId,
    id: &DestinationId,
    name: Option<String>,
    file: Option<String>,
) -> Result<CommandResult, CliError> {
    let input = match file {
        Some(file) => UpdateDestinationInput::try_from(read_json_input(&file)?)?,
        None => UpdateDestinationInput::rename(
            api.get_destination(workspace, id).await?.data(),
            name.ok_or_else(|| CliError::InvalidInput("supply --name or --file".into()))?,
        )?,
    };
    api.update_destination(workspace, id, input)
        .await
        .map(CommandResult::DestinationUpdated)
}
