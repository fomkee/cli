use crate::client::IncidentApi;
use crate::commands::PageArgs;
use crate::dto::incidents::PostNoteInput;
use crate::error::CliError;
use crate::model::{IncidentId, MonitorId, WorkspaceId};
use crate::monitoring::read_text_input;
use crate::result::CommandResult;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(crate) enum IncidentCommand {
    /// List incidents and their current status.
    List {
        /// Show incidents for this monitor only.
        #[arg(long)]
        monitor: Option<MonitorId>,
        #[command(flatten)]
        page: PageArgs,
    },
    /// Show an incident's status and affected monitor.
    Get {
        /// Incident ID shown by incident list.
        incident_id: IncidentId,
    },
    /// Show incident history, including published notes.
    Timeline {
        /// Incident ID shown by incident list.
        incident_id: IncidentId,
        #[command(flatten)]
        page: PageArgs,
    },
    /// Publish a public-facing note on an incident.
    #[command(
        after_help = "Notes appear on status pages that include this incident's monitor. Posting a note does not change the incident's status."
    )]
    Post {
        /// Incident ID shown by incident list.
        incident_id: IncidentId,
        /// Public note text, from 1 to 2000 characters.
        #[arg(
            long,
            required_unless_present = "message_file",
            conflicts_with = "message_file"
        )]
        message: Option<String>,
        /// Read public note text from a file; use - for standard input.
        #[arg(long)]
        message_file: Option<String>,
    },
}

pub(crate) async fn execute(
    api: &impl IncidentApi,
    workspace: &WorkspaceId,
    command: IncidentCommand,
) -> Result<CommandResult, CliError> {
    match command {
        IncidentCommand::List { monitor, page } => api
            .list_incidents(
                workspace,
                monitor.as_ref(),
                page.limit,
                page.after.as_deref(),
            )
            .await
            .map(CommandResult::IncidentList),
        IncidentCommand::Get { incident_id } => api
            .get_incident(workspace, &incident_id)
            .await
            .map(CommandResult::Incident),
        IncidentCommand::Timeline { incident_id, page } => api
            .incident_timeline(workspace, &incident_id, page.limit, page.after.as_deref())
            .await
            .map(CommandResult::IncidentTimeline),
        IncidentCommand::Post {
            incident_id,
            message,
            message_file,
        } => {
            let input = note_input(message, message_file)?;
            api.post_incident_note(workspace, &incident_id, input)
                .await
                .map(CommandResult::IncidentNote)
        }
    }
}

fn note_input(message: Option<String>, file: Option<String>) -> Result<PostNoteInput, CliError> {
    let message = match (message, file) {
        (Some(message), None) => message,
        (None, Some(path)) => read_text_input(&path)?,
        _ => {
            return Err(CliError::InvalidInput(
                "provide --message or --message-file".into(),
            ));
        }
    };
    Ok(PostNoteInput { message })
}
