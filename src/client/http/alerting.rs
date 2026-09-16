use super::HttpFomkeeApi;
use super::{page_query, serialize_body};
use crate::alerting::AssignmentScope;
use crate::client::AlertingApi;
use crate::dto::alerting::{
    Assignment, CreateDestinationInput, Destination, DestinationTest, Page, UpdateDestinationInput,
};
use crate::error::CliError;
use crate::model::{AssignmentId, DestinationId, MonitorId, WorkspaceId};
use crate::wire::Response;
use async_trait::async_trait;
use reqwest::Method;
use serde_json::json;
#[async_trait]
impl AlertingApi for HttpFomkeeApi {
    async fn list_destinations(
        &self,
        workspace: &WorkspaceId,
        limit: u32,
        after: Option<&str>,
    ) -> Result<Response<Page<Destination>>, CliError> {
        self.read_request(&format!(
            "api/workspaces/{workspace}/alert-targets?{}",
            page_query(limit, after)?
        ))
        .await
    }
    async fn get_destination(
        &self,
        workspace: &WorkspaceId,
        id: &DestinationId,
    ) -> Result<Response<Destination>, CliError> {
        self.read_request(&format!("api/workspaces/{workspace}/alert-targets/{id}"))
            .await
    }
    async fn create_destination(
        &self,
        workspace: &WorkspaceId,
        input: CreateDestinationInput,
    ) -> Result<Response<Destination>, CliError> {
        self.request(
            Method::POST,
            &format!("api/workspaces/{workspace}/alert-targets"),
            Some(serialize_body(&input)?),
            true,
        )
        .await
    }
    async fn update_destination(
        &self,
        workspace: &WorkspaceId,
        id: &DestinationId,
        input: UpdateDestinationInput,
    ) -> Result<Response<Destination>, CliError> {
        self.request(
            Method::PUT,
            &format!("api/workspaces/{workspace}/alert-targets/{id}"),
            Some(serialize_body(&input)?),
            true,
        )
        .await
    }
    async fn test_destination(
        &self,
        workspace: &WorkspaceId,
        id: &DestinationId,
    ) -> Result<Response<DestinationTest>, CliError> {
        self.request(
            Method::POST,
            &format!("api/workspaces/{workspace}/alert-targets/{id}/test"),
            None,
            true,
        )
        .await
    }
    async fn list_assignments(
        &self,
        workspace: &WorkspaceId,
        scope: AssignmentScope,
        limit: u32,
        after: Option<&str>,
    ) -> Result<Response<Page<Assignment>>, CliError> {
        let path = match scope {
            AssignmentScope::Monitor(id) => format!("monitors/{id}/alert-assignments"),
            AssignmentScope::Destination(id) => format!("alert-targets/{id}/alert-assignments"),
        };
        self.read_request(&format!(
            "api/workspaces/{workspace}/{path}?{}",
            page_query(limit, after)?
        ))
        .await
    }
    async fn assign_destination(
        &self,
        workspace: &WorkspaceId,
        monitor: &MonitorId,
        destination: &DestinationId,
    ) -> Result<Response<Assignment>, CliError> {
        self.request(
            Method::POST,
            &format!("api/workspaces/{workspace}/monitors/{monitor}/alert-assignments"),
            Some(json!({"alert_target_id": destination})),
            true,
        )
        .await
    }
    async fn remove_assignment(
        &self,
        workspace: &WorkspaceId,
        assignment: &AssignmentId,
    ) -> Result<(), CliError> {
        self.request::<()>(
            Method::DELETE,
            &format!("api/workspaces/{workspace}/alert-assignments/{assignment}"),
            None,
            true,
        )
        .await
        .map(|_| ())
    }
}
