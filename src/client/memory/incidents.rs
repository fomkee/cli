use super::{InMemoryFomkeeApi, unconfigured};
use crate::client::IncidentApi;
use crate::dto::alerting::Page;
use crate::dto::incidents::{Incident, IncidentEvent, PostNoteInput, PublishedNote};
use crate::error::CliError;
use crate::model::{IncidentId, MonitorId, WorkspaceId};
use crate::wire::Response;
use async_trait::async_trait;
#[async_trait]
impl IncidentApi for InMemoryFomkeeApi {
    async fn list_incidents(
        &self,
        _: &WorkspaceId,
        _: Option<&MonitorId>,
        _: u32,
        _: Option<&str>,
    ) -> Result<Response<Page<Incident>>, CliError> {
        unconfigured()
    }
    async fn get_incident(
        &self,
        _: &WorkspaceId,
        _: &IncidentId,
    ) -> Result<Response<Incident>, CliError> {
        unconfigured()
    }
    async fn incident_timeline(
        &self,
        _: &WorkspaceId,
        _: &IncidentId,
        _: u32,
        _: Option<&str>,
    ) -> Result<Response<Page<IncidentEvent>>, CliError> {
        unconfigured()
    }
    async fn post_incident_note(
        &self,
        _: &WorkspaceId,
        _: &IncidentId,
        _: PostNoteInput,
    ) -> Result<Response<PublishedNote>, CliError> {
        unconfigured()
    }
}
