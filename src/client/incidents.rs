use crate::dto::alerting::Page;
use crate::dto::incidents::{Incident, IncidentEvent, PostNoteInput, PublishedNote};
use crate::error::CliError;
use crate::model::{IncidentId, MonitorId, WorkspaceId};
use crate::wire::Response;
use async_trait::async_trait;

/// Public incidents operations.
#[async_trait]
pub trait IncidentApi: Send + Sync {
    /// Return one incident page, optionally for a monitor.
    async fn list_incidents(
        &self,
        workspace: &WorkspaceId,
        monitor: Option<&MonitorId>,
        limit: u32,
        after: Option<&str>,
    ) -> Result<Response<Page<Incident>>, CliError>;
    /// Fetch incident status and monitor identity.
    async fn get_incident(
        &self,
        workspace: &WorkspaceId,
        id: &IncidentId,
    ) -> Result<Response<Incident>, CliError>;
    /// Return one page of incident history.
    async fn incident_timeline(
        &self,
        workspace: &WorkspaceId,
        id: &IncidentId,
        limit: u32,
        after: Option<&str>,
    ) -> Result<Response<Page<IncidentEvent>>, CliError>;
    /// Publish one public-facing incident note without retrying.
    async fn post_incident_note(
        &self,
        workspace: &WorkspaceId,
        id: &IncidentId,
        input: PostNoteInput,
    ) -> Result<Response<PublishedNote>, CliError>;
}
