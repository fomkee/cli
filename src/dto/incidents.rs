use crate::model::{IncidentEventId, IncidentId, MonitorId, WorkspaceId};
use serde::{Deserialize, Serialize};

/// Incident summary returned by collection and detail endpoints.
#[derive(Clone, Deserialize)]
pub struct Incident {
    pub id: IncidentId,
    pub monitor_id: MonitorId,
    pub workspace_id: WorkspaceId,
    pub state: IncidentState,
    pub monitor_name: String,
    pub monitor_type: String,
    pub created_at: String,
    pub resolved_at: Option<String>,
    pub confirming_nodes: Vec<String>,
    pub regression_count: u64,
}
/// API-owned incident lifecycle.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentState {
    Open,
    Regressed,
    Resolved,
}
impl IncidentState {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::Regressed => "Regressed",
            Self::Resolved => "Resolved",
        }
    }
}
/// Timeline entry with a typed event discriminator; JSON retains full evidence.
#[derive(Clone, Deserialize)]
pub struct IncidentEvent {
    pub id: IncidentEventId,
    pub occurred_at: String,
    #[serde(flatten)]
    pub event: EventKind,
}
/// Supported timeline events.
#[derive(Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    Opened,
    NodeConfirmed,
    NodeRecovered,
    Regressed,
    AlertRequested,
    Resolved,
    StatusUpdatePosted { message: String, posted_by: String },
}
impl EventKind {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Opened => "Opened",
            Self::NodeConfirmed => "Node confirmed failure",
            Self::NodeRecovered => "Node recovered",
            Self::Regressed => "Regressed",
            Self::AlertRequested => "Alert requested",
            Self::Resolved => "Resolved",
            Self::StatusUpdatePosted { .. } => "Public note",
        }
    }
}
/// Successful publication must return a status-update event.
#[derive(Clone, Deserialize)]
pub struct PublishedNote {
    pub id: IncidentEventId,
    pub occurred_at: String,
    pub message: String,
    pub posted_by: String,
    #[serde(rename = "type")]
    pub kind: PublishedNoteKind,
}
/// Expected publication result discriminator.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishedNoteKind {
    StatusUpdatePosted,
}
/// Public-facing text to append to an incident.
#[derive(Serialize)]
pub struct PostNoteInput {
    pub message: String,
}
