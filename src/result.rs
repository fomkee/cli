use crate::alerting::RemovedAssignment;
use crate::dto::alerting::{Assignment, Destination, DestinationTest, Page};
use crate::dto::incidents::{Incident, IncidentEvent, PublishedNote};
use crate::dto::maintenance::Maintenance;
use crate::update::UpdateResult;
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
    Updated,
    Paused,
    Resumed,
    Disabled,
    Enabled,
}

impl MonitorAction {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Created => "Created",
            Self::Updated => "Updated",
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
    IncidentList(Response<Page<Incident>>),
    Incident(Response<Incident>),
    IncidentTimeline(Response<Page<IncidentEvent>>),
    IncidentNote(Response<PublishedNote>),
    MaintenanceList(Response<Page<Maintenance>>),
    MaintenanceCreated(Response<Maintenance>),
    MaintenanceCancelled(Response<Maintenance>),
    SelfUpdate(UpdateResult),
    DestinationList(Response<Page<Destination>>),
    Destination(Response<Destination>),
    DestinationCreated(Response<Destination>),
    DestinationUpdated(Response<Destination>),
    DestinationTest(Response<DestinationTest>),
    Assignments(Response<Page<Assignment>>),
    Assigned(Response<Assignment>),
    Unassigned(RemovedAssignment),
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
