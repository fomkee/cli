use super::{HttpFomkeeApi, page_query, serialize_body};
use crate::client::IncidentApi;
use crate::dto::alerting::Page;
use crate::dto::incidents::{Incident, IncidentEvent, PostNoteInput, PublishedNote};
use crate::error::CliError;
use crate::model::{IncidentId, MonitorId, WorkspaceId};
use crate::wire::Response;
use async_trait::async_trait;
use reqwest::Method;
#[async_trait]
impl IncidentApi for HttpFomkeeApi {
    async fn list_incidents(
        &self,
        workspace: &WorkspaceId,
        monitor: Option<&MonitorId>,
        limit: u32,
        after: Option<&str>,
    ) -> Result<Response<Page<Incident>>, CliError> {
        let scope = monitor
            .map(|id| format!("monitors/{id}/incidents"))
            .unwrap_or_else(|| "incidents".into());
        self.read_request(&format!(
            "api/workspaces/{workspace}/{scope}?{}",
            page_query(limit, after)?
        ))
        .await
    }
    async fn get_incident(
        &self,
        workspace: &WorkspaceId,
        id: &IncidentId,
    ) -> Result<Response<Incident>, CliError> {
        self.read_request(&format!("api/workspaces/{workspace}/incidents/{id}"))
            .await
    }
    async fn incident_timeline(
        &self,
        workspace: &WorkspaceId,
        id: &IncidentId,
        limit: u32,
        after: Option<&str>,
    ) -> Result<Response<Page<IncidentEvent>>, CliError> {
        self.read_request(&format!(
            "api/workspaces/{workspace}/incidents/{id}/timeline?{}",
            page_query(limit, after)?
        ))
        .await
    }
    async fn post_incident_note(
        &self,
        workspace: &WorkspaceId,
        id: &IncidentId,
        input: PostNoteInput,
    ) -> Result<Response<PublishedNote>, CliError> {
        self.request(
            Method::POST,
            &format!("api/workspaces/{workspace}/incidents/{id}/updates"),
            Some(serialize_body(&input)?),
            true,
        )
        .await
    }
}
