use crate::error::CliError;
use crate::model::{MaintenanceId, MonitorId, WorkspaceId};
use crate::wire::Response;
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Scheduled maintenance as returned by the API.
#[derive(Clone, Deserialize)]
pub struct Maintenance {
    pub id: MaintenanceId,
    pub workspace_id: WorkspaceId,
    pub title: String,
    pub description: Option<String>,
    pub scheduled_start: String,
    pub scheduled_end: String,
    pub monitors: Vec<MonitorId>,
    pub state: MaintenanceState,
    pub created_at: String,
    pub created_by: String,
}
/// API-owned maintenance lifecycle.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceState {
    Scheduled,
    InProgress,
    Completed,
    Cancelled,
}
impl MaintenanceState {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Scheduled => "Scheduled",
            Self::InProgress => "In progress",
            Self::Completed => "Completed",
            Self::Cancelled => "Cancelled",
        }
    }
}
/// Complete maintenance input, preserving original file fields.
#[derive(Serialize)]
#[serde(transparent)]
pub struct CreateMaintenanceInput(Response<MaintenanceRequest>);
/// Wire shape of complete maintenance creation settings.
#[derive(Deserialize)]
pub struct MaintenanceRequest {
    pub title: String,
    pub description: Option<String>,
    pub scheduled_start: String,
    pub scheduled_end: String,
    pub monitors: Vec<MonitorId>,
}
impl TryFrom<Value> for CreateMaintenanceInput {
    type Error = CliError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let response = Response::<MaintenanceRequest>::parse(value).map_err(|_| {
            CliError::InvalidInput(
                "maintenance settings need title, scheduled_start, scheduled_end, and monitor IDs"
                    .into(),
            )
        })?;
        timestamp_input(&response.data().scheduled_start)?;
        timestamp_input(&response.data().scheduled_end)?;
        Ok(Self(response))
    }
}
pub(crate) fn timestamp_input(value: &str) -> Result<String, CliError> {
    DateTime::parse_from_rfc3339(value).map_err(|_| {
        CliError::InvalidInput(
            "time must include a date and timezone, e.g. 2026-10-01T09:00:00Z".into(),
        )
    })?;
    Ok(value.to_owned())
}
