use serde::Serialize;

use crate::dto::dry_run::DryRun;
use crate::dto::{CreatedMonitor, Entitlements, Monitor, MonitorPage};
use crate::model::MonitorId;
use crate::skill::ExportedSkills;
use crate::wire::Response;
use crate::workspace::output::{
    AuthStatus, ConfigInfo, ConfigPaths, Disconnected, ProfileOutput, WorkspaceList,
};

#[derive(Clone, Copy)]
pub(crate) enum MonitorAction {
    Created,
    Paused,
    Resumed,
    Disabled,
    Enabled,
}

impl MonitorAction {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Created => "Created",
            Self::Paused => "Paused",
            Self::Resumed => "Resumed",
            Self::Disabled => "Disabled",
            Self::Enabled => "Enabled",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum WorkspaceAction {
    Connected,
    Selected,
}

impl WorkspaceAction {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Connected => "Connected",
            Self::Selected => "Selected",
        }
    }
}

#[derive(Serialize)]
pub(crate) struct ActionResult<A, T> {
    #[serde(skip)]
    pub(crate) action: A,
    #[serde(flatten)]
    pub(crate) result: T,
}

#[derive(Serialize)]
pub(crate) struct DeletedMonitor {
    pub(crate) deleted: bool,
    pub(crate) monitor_id: MonitorId,
}

/// The command and its data cannot disagree; JSON retains the existing envelope.
#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum CommandResult {
    MonitorList(Response<MonitorPage>),
    MonitorGet(Response<Monitor>),
    Created(Response<CreatedMonitor>),
    MonitorAction(ActionResult<MonitorAction, Response<Monitor>>),
    Deleted(DeletedMonitor),
    DryRun(Response<DryRun>),
    WorkspaceList(WorkspaceList),
    WorkspaceAction(ActionResult<WorkspaceAction, ProfileOutput>),
    Disconnected(Disconnected),
    Auth(AuthStatus),
    ConfigPaths(ConfigPaths),
    ConfigShow(ConfigInfo),
    SkillExport(ExportedSkills),
    Entitlement(Response<Entitlements>),
}
