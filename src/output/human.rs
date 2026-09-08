use super::context::DisplayContext;
use super::format::text;
use super::layout::Ui;
use super::theme::Verdict;
use super::{connection, dry_run, monitor, skill};
use crate::error::CliError;
use crate::error::response::ResponseOutcome;
use crate::result::CommandResult;

pub(super) fn render(
    result: &CommandResult,
    details: bool,
    width: u16,
    color: bool,
    context: &DisplayContext,
) -> String {
    let mut ui = Ui::new(width, color);
    match result {
        CommandResult::MonitorGet(value) => {
            ui.page_width();
            monitor::detail(&mut ui, value.data(), details, context);
            ui.finish_page()
        }
        CommandResult::Created(value) => {
            ui.page_width();
            monitor::created(&mut ui, value.data(), details, context);
            ui.finish_page()
        }
        CommandResult::MonitorAction(value) => {
            ui.page_width();
            monitor::action(&mut ui, value.result.data(), value.action, details, context);
            ui.finish_page()
        }
        CommandResult::Deleted(value) => {
            ui.page_width();
            ui.verdict(
                &format!("Deleted monitor {}.", value.monitor_id),
                Verdict::Success,
            );
            ui.finish_page()
        }
        CommandResult::MonitorList(value) => {
            monitor::list(&mut ui, value.data(), details, context);
            ui.finish()
        }
        CommandResult::DryRun(value) => {
            dry_run::render(&mut ui, value.data(), details);
            ui.finish()
        }
        CommandResult::WorkspaceList(value) => {
            connection::list(&mut ui, value, details);
            ui.finish()
        }
        CommandResult::WorkspaceAction(value) => {
            connection::action(&mut ui, &value.result, value.action, details);
            ui.finish()
        }
        CommandResult::Disconnected(value) => {
            connection::disconnected(&mut ui, value);
            ui.finish()
        }
        CommandResult::Auth(value) => {
            connection::auth(&mut ui, value, details);
            ui.finish()
        }
        CommandResult::ConfigPaths(value) => {
            connection::paths(&mut ui, value);
            ui.finish()
        }
        CommandResult::ConfigShow(value) => {
            connection::config(&mut ui, value);
            ui.finish()
        }
        CommandResult::SkillExport(value) => {
            skill::export(&mut ui, value);
            ui.finish()
        }
        CommandResult::Entitlement(value) => {
            connection::entitlement(&mut ui, value.data(), details);
            ui.finish()
        }
    }
}

pub(super) fn error(error: &CliError, width: u16, color: bool) -> String {
    let mut ui = Ui::new(width, color);
    ui.verdict("Error", Verdict::Failure);
    ui.line(&text(&error.to_string()));
    if let CliError::Api {
        details: Some(value),
        ..
    } = error
    {
        ui.object("Details", value);
    }
    if let CliError::Api {
        retry_after_secs: Some(seconds),
        ..
    } = error
    {
        ui.hint(&format!("Retry after {seconds} seconds."));
    }
    let hint = match error {
        CliError::MissingCredentials => Some("Run fomkeecli workspace connect ALIAS."),
        CliError::Configuration(_) => {
            Some("Run fomkeecli config paths to locate the configuration.")
        }
        CliError::OutcomeUnknown(_) => {
            Some("Run fomkeecli monitor list and inspect the target before retrying.")
        }
        CliError::Api {
            status: 401 | 403, ..
        } => Some("Check the selected workspace and token access with fomkeecli auth status."),
        CliError::Response(error) => match error.outcome {
            ResponseOutcome::Read => None,
            ResponseOutcome::MutationAcknowledged | ResponseOutcome::MutationErrorReported => {
                Some("Run fomkeecli monitor list and inspect the target before retrying.")
            }
        },
        CliError::CredentialStore
        | CliError::MissingSavedCredential
        | CliError::SkillExport(_)
        | CliError::InvalidInput(_)
        | CliError::ConfirmationRequired
        | CliError::Api { .. }
        | CliError::Transport(_)
        | CliError::MalformedResponse(_) => None,
    };
    if let Some(hint) = hint {
        ui.hint(hint);
    }
    ui.finish()
}

#[cfg(test)]
mod context_tests;
#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod tests;
