use super::{InMemoryFomkeeApi, unconfigured};
use crate::client::MaintenanceApi;
use crate::dto::alerting::Page;
use crate::dto::maintenance::{CreateMaintenanceInput, Maintenance};
use crate::error::CliError;
use crate::model::{MaintenanceId, WorkspaceId};
use crate::wire::Response;
use async_trait::async_trait;
#[async_trait]
impl MaintenanceApi for InMemoryFomkeeApi {
    async fn list_maintenance(
        &self,
        _: &WorkspaceId,
        _: u32,
        _: Option<&str>,
    ) -> Result<Response<Page<Maintenance>>, CliError> {
        unconfigured()
    }
    async fn create_maintenance(
        &self,
        _: &WorkspaceId,
        _: CreateMaintenanceInput,
    ) -> Result<Response<Maintenance>, CliError> {
        unconfigured()
    }
    async fn cancel_maintenance(
        &self,
        _: &WorkspaceId,
        _: &MaintenanceId,
    ) -> Result<Response<Maintenance>, CliError> {
        unconfigured()
    }
}
