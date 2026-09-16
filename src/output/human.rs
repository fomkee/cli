use super::context::DisplayContext;
use super::format::text;
use super::layout::Ui;
use super::theme::Verdict;
use super::{alerting, connection, dry_run, incidents, maintenance, monitor, skill};
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
        CommandResult::IncidentList(value) => {
            incidents::list(&mut ui, value.data(), details);
            ui.finish()
        }
        CommandResult::Incident(value) => {
            incidents::detail(&mut ui, value.data(), details);
            ui.finish()
        }
        CommandResult::IncidentTimeline(value) => {
            incidents::timeline(&mut ui, value.data());
            ui.finish()
        }
        CommandResult::IncidentNote(value) => {
            incidents::published(&mut ui, value.data());
            ui.finish()
        }
        CommandResult::MaintenanceList(value) => {
            maintenance::list(&mut ui, value.data(), details);
            ui.finish()
        }
        CommandResult::MaintenanceCreated(value) => {
            maintenance::detail(&mut ui, value.data(), "Scheduled maintenance", details);
            ui.finish()
        }
        CommandResult::MaintenanceCancelled(value) => {
            maintenance::detail(&mut ui, value.data(), "Cancelled maintenance", details);
            ui.finish()
        }
        CommandResult::SelfUpdate(value) => {
            ui.title(if value.updated {
                "Updated fomkeecli"
            } else {
                "fomkeecli version check"
            });
            ui.fields(vec![
                ("Previously installed".into(), text(&value.current_version)),
                ("Latest stable".into(), text(&value.latest_version)),
            ]);
            if value.updated {
                ui.line("The next invocation uses the new version.");
            } else if value.update_available {
                ui.hint("Run fomkeecli self-update to install this release.");
            } else {
                ui.line("No newer stable release is available.");
            }
            ui.finish()
        }

        CommandResult::DestinationList(value) => {
            alerting::list(&mut ui, value.data(), details);
            ui.finish()
        }
        CommandResult::Destination(value) => {
            alerting::destination(&mut ui, value.data(), "Alert destination", details);
            ui.finish()
        }
        CommandResult::DestinationCreated(value) => {
            alerting::destination(&mut ui, value.data(), "Created alert destination", details);
            ui.finish()
        }
        CommandResult::DestinationUpdated(value) => {
            alerting::destination(&mut ui, value.data(), "Updated alert destination", details);
            ui.finish()
        }
        CommandResult::DestinationTest(value) => {
            alerting::test(&mut ui, value.data());
            ui.finish()
        }
        CommandResult::Assignments(value) => {
            alerting::assignments(&mut ui, value.data());
            ui.finish()
        }
        CommandResult::Assigned(value) => {
            ui.title("Assigned alert destination");
            alerting::assignment(&mut ui, value.data());
            ui.finish()
        }
        CommandResult::Unassigned(value) => {
            ui.title("Removed alert assignment");
            ui.fields(vec![(
                "Assignment ID".into(),
                value.assignment_id.to_string(),
            )]);
            ui.finish()
        }

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
        CliError::OutcomeUnknown(_) => Some(
            "Inspect current state with fomkeecli monitor list or fomkeecli destination list before retrying.",
        ),
        CliError::Api {
            status: 401 | 403, ..
        } => Some("Check the selected workspace and token access with fomkeecli auth status."),
        CliError::Response(error) => match error.outcome {
            ResponseOutcome::Read => None,
            ResponseOutcome::MutationAcknowledged | ResponseOutcome::MutationErrorReported => Some(
                "Inspect current state with fomkeecli monitor list or fomkeecli destination list before retrying.",
            ),
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
