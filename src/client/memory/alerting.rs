use super::InMemoryFomkeeApi;
use super::unconfigured;
use crate::alerting::AssignmentScope;
use crate::client::AlertingApi;
use crate::dto::alerting::{
    Assignment, CreateDestinationInput, Destination, DestinationTest, Page, UpdateDestinationInput,
};
use crate::error::CliError;
use crate::model::{AssignmentId, DestinationId, MonitorId, WorkspaceId};
use crate::wire::Response;
use async_trait::async_trait;
#[async_trait]
impl AlertingApi for InMemoryFomkeeApi {
    async fn list_destinations(
        &self,
        _: &WorkspaceId,
        _: u32,
        _: Option<&str>,
    ) -> Result<Response<Page<Destination>>, CliError> {
        unconfigured()
    }
    async fn get_destination(
        &self,
        _: &WorkspaceId,
        _: &DestinationId,
    ) -> Result<Response<Destination>, CliError> {
        unconfigured()
    }
    async fn create_destination(
        &self,
        _: &WorkspaceId,
        _: CreateDestinationInput,
    ) -> Result<Response<Destination>, CliError> {
        unconfigured()
    }
    async fn update_destination(
        &self,
        _: &WorkspaceId,
        _: &DestinationId,
        _: UpdateDestinationInput,
    ) -> Result<Response<Destination>, CliError> {
        unconfigured()
    }
    async fn test_destination(
        &self,
        _: &WorkspaceId,
        _: &DestinationId,
    ) -> Result<Response<DestinationTest>, CliError> {
        unconfigured()
    }
    async fn list_assignments(
        &self,
        _: &WorkspaceId,
        _: AssignmentScope,
        _: u32,
        _: Option<&str>,
    ) -> Result<Response<Page<Assignment>>, CliError> {
        unconfigured()
    }
    async fn assign_destination(
        &self,
        _: &WorkspaceId,
        _: &MonitorId,
        _: &DestinationId,
    ) -> Result<Response<Assignment>, CliError> {
        unconfigured()
    }
    async fn remove_assignment(&self, _: &WorkspaceId, _: &AssignmentId) -> Result<(), CliError> {
        unconfigured()
    }
}
