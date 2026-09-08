use serde_json::Value;

use super::{CommandResult, DisplayContext};
use crate::result::{ActionResult, MonitorAction, WorkspaceAction};
use crate::wire::Response;

pub(super) enum Fixture {
    MonitorGet,
    MonitorList,
    Created,
    Paused,
    DryRun,
    Entitlement,
    Disconnected,
    Connected,
}

pub(super) fn render(
    value: &Value,
    fixture: Fixture,
    details: bool,
    width: u16,
    color: bool,
    context: &DisplayContext,
) -> String {
    let result = match fixture {
        Fixture::MonitorGet => CommandResult::MonitorGet(Response::parse(value.clone()).unwrap()),
        Fixture::MonitorList => CommandResult::MonitorList(Response::parse(value.clone()).unwrap()),
        Fixture::Created => CommandResult::Created(Response::parse(value.clone()).unwrap()),
        Fixture::Paused => CommandResult::MonitorAction(ActionResult {
            action: MonitorAction::Paused,
            result: Response::parse(value.clone()).unwrap(),
        }),
        Fixture::DryRun => CommandResult::DryRun(Response::parse(value.clone()).unwrap()),
        Fixture::Entitlement => CommandResult::Entitlement(Response::parse(value.clone()).unwrap()),
        Fixture::Disconnected => {
            CommandResult::Disconnected(serde_json::from_value(value.clone()).unwrap())
        }
        Fixture::Connected => CommandResult::WorkspaceAction(ActionResult {
            action: WorkspaceAction::Connected,
            result: serde_json::from_value(value.clone()).unwrap(),
        }),
    };
    super::render(&result, details, width, color, context)
}
