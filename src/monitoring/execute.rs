use super::{CreateCommand, DryRunInput, LifecycleInput, read_json_input};
use crate::alerting::AssignmentScope;
use crate::client::FomkeeApi;
use crate::commands::MonitorCommand;
use crate::error::CliError;
use crate::model::{MonitorId, WorkspaceId};
use crate::output::{Output, OutputMode};
use crate::result::{ActionResult, CommandResult, DeletedMonitor, MonitorAction};

pub(crate) async fn monitor_command(
    api: &impl FomkeeApi,
    command: MonitorCommand,
    workspace: WorkspaceId,
    output: OutputMode,
) -> Result<CommandResult, CliError> {
    match command {
        MonitorCommand::Destinations {
            monitor_id,
            limit,
            after,
        } => api
            .list_assignments(
                &workspace,
                AssignmentScope::Monitor(monitor_id),
                limit,
                after.as_deref(),
            )
            .await
            .map(CommandResult::Assignments),
        MonitorCommand::Assign {
            monitor_id,
            destination_id,
        } => api
            .assign_destination(&workspace, &monitor_id, &destination_id)
            .await
            .map(CommandResult::Assigned),
        MonitorCommand::Update {
            monitor_id,
            changes,
        } => super::update::execute(api, &workspace, &monitor_id, *changes)
            .await
            .map(|result| {
                CommandResult::MonitorAction(ActionResult {
                    action: MonitorAction::Updated,
                    result,
                })
            }),
        MonitorCommand::List { limit, after } => api
            .list_monitors(&workspace, limit, after.as_deref())
            .await
            .map(CommandResult::MonitorList),
        MonitorCommand::Get { monitor_id } => api
            .get_monitor(&workspace, &monitor_id)
            .await
            .map(CommandResult::MonitorGet),
        MonitorCommand::DryRun(input) => dry_run(api, &workspace, &input.file).await,
        MonitorCommand::Create { command } => create(api, &workspace, command).await,
        MonitorCommand::Pause {
            monitor_id,
            reason,
            resume_at,
        } => {
            lifecycle(
                api,
                &workspace,
                &monitor_id,
                LifecycleInput::pause(reason, resume_at),
            )
            .await
        }
        MonitorCommand::Resume { monitor_id } => {
            lifecycle(api, &workspace, &monitor_id, LifecycleInput::resume()).await
        }
        MonitorCommand::Disable { monitor_id, reason } => {
            lifecycle(
                api,
                &workspace,
                &monitor_id,
                LifecycleInput::disable(reason),
            )
            .await
        }
        MonitorCommand::Enable { monitor_id } => {
            lifecycle(api, &workspace, &monitor_id, LifecycleInput::enable()).await
        }
        MonitorCommand::Delete { monitor_id, yes } => {
            delete(api, &workspace, monitor_id, yes, output).await
        }
    }
}

async fn dry_run(
    api: &impl FomkeeApi,
    workspace: &WorkspaceId,
    file: &str,
) -> Result<CommandResult, CliError> {
    let input = DryRunInput::try_from(read_json_input(file)?)?;
    api.dry_run(workspace, input)
        .await
        .map(CommandResult::DryRun)
}

async fn create(
    api: &impl FomkeeApi,
    workspace: &WorkspaceId,
    command: CreateCommand,
) -> Result<CommandResult, CliError> {
    let needs_interval = command.needs_interval();
    let mut input = command.into_input()?;
    if needs_interval {
        input.use_api_interval(api.entitlements(workspace).await?.data())?;
    }
    api.create_monitor(workspace, input)
        .await
        .map(CommandResult::Created)
}

async fn lifecycle(
    api: &impl FomkeeApi,
    workspace: &WorkspaceId,
    monitor: &MonitorId,
    input: LifecycleInput,
) -> Result<CommandResult, CliError> {
    let action = match &input {
        LifecycleInput::Pause(_) => MonitorAction::Paused,
        LifecycleInput::Resume => MonitorAction::Resumed,
        LifecycleInput::Disable(_) => MonitorAction::Disabled,
        LifecycleInput::Enable => MonitorAction::Enabled,
    };
    let result = api.lifecycle(workspace, monitor, input).await?;
    Ok(CommandResult::MonitorAction(ActionResult {
        action,
        result,
    }))
}

async fn delete(
    api: &impl FomkeeApi,
    workspace: &WorkspaceId,
    monitor_id: MonitorId,
    yes: bool,
    output: OutputMode,
) -> Result<CommandResult, CliError> {
    if !yes {
        match output {
            OutputMode::Json => return Err(CliError::ConfirmationRequired),
            OutputMode::Human => {
                if !Output::confirm_delete(&monitor_id.to_string())? {
                    return Err(CliError::ConfirmationRequired);
                }
            }
        }
    }
    api.delete_monitor(workspace, &monitor_id).await?;
    Ok(CommandResult::Deleted(DeletedMonitor {
        deleted: true,
        monitor_id,
    }))
}
