use super::{HttpFomkeeApi, page_query, serialize_body};
use crate::client::MaintenanceApi;
use crate::dto::alerting::Page;
use crate::dto::maintenance::{CreateMaintenanceInput, Maintenance};
use crate::error::CliError;
use crate::model::{MaintenanceId, WorkspaceId};
use crate::wire::Response;
use async_trait::async_trait;
use reqwest::Method;
#[async_trait]
impl MaintenanceApi for HttpFomkeeApi {
    async fn list_maintenance(
        &self,
        workspace: &WorkspaceId,
        limit: u32,
        after: Option<&str>,
    ) -> Result<Response<Page<Maintenance>>, CliError> {
        self.read_request(&format!(
            "api/workspaces/{workspace}/maintenance?{}",
            page_query(limit, after)?
        ))
        .await
    }
    async fn create_maintenance(
        &self,
        workspace: &WorkspaceId,
        input: CreateMaintenanceInput,
    ) -> Result<Response<Maintenance>, CliError> {
        self.request(
            Method::POST,
            &format!("api/workspaces/{workspace}/maintenance"),
            Some(serialize_body(&input)?),
            true,
        )
        .await
    }
    async fn cancel_maintenance(
        &self,
        workspace: &WorkspaceId,
        id: &MaintenanceId,
    ) -> Result<Response<Maintenance>, CliError> {
        self.request(
            Method::POST,
            &format!("api/workspaces/{workspace}/maintenance/{id}/cancel"),
            None,
            true,
        )
        .await
    }
}
