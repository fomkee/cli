use crate::alerting::AssignmentScope;
use crate::dto::alerting::{
    Assignment, CreateDestinationInput, Destination, DestinationTest, Page, UpdateDestinationInput,
};
use crate::error::CliError;
use crate::model::{AssignmentId, DestinationId, MonitorId, WorkspaceId};
use crate::wire::Response;
use async_trait::async_trait;

/// Public destination and assignment operations; mutations are never retried.
#[async_trait]
pub trait AlertingApi: Send + Sync {
    /// Return one destination page.
    async fn list_destinations(
        &self,
        workspace: &WorkspaceId,
        limit: u32,
        after: Option<&str>,
    ) -> Result<Response<Page<Destination>>, CliError>;
    /// Fetch safe destination metadata.
    async fn get_destination(
        &self,
        workspace: &WorkspaceId,
        id: &DestinationId,
    ) -> Result<Response<Destination>, CliError>;
    /// Create one destination.
    async fn create_destination(
        &self,
        workspace: &WorkspaceId,
        input: CreateDestinationInput,
    ) -> Result<Response<Destination>, CliError>;
    /// Replace a destination with explicit secret actions.
    async fn update_destination(
        &self,
        workspace: &WorkspaceId,
        id: &DestinationId,
        input: UpdateDestinationInput,
    ) -> Result<Response<Destination>, CliError>;
    /// Send a real test notification once.
    async fn test_destination(
        &self,
        workspace: &WorkspaceId,
        id: &DestinationId,
    ) -> Result<Response<DestinationTest>, CliError>;
    /// List one assignment page for the selected scope.
    async fn list_assignments(
        &self,
        workspace: &WorkspaceId,
        scope: AssignmentScope,
        limit: u32,
        after: Option<&str>,
    ) -> Result<Response<Page<Assignment>>, CliError>;
    /// Create one monitor binding.
    async fn assign_destination(
        &self,
        workspace: &WorkspaceId,
        monitor: &MonitorId,
        destination: &DestinationId,
    ) -> Result<Response<Assignment>, CliError>;
    /// Remove one binding by its assignment ID.
    async fn remove_assignment(
        &self,
        workspace: &WorkspaceId,
        assignment: &AssignmentId,
    ) -> Result<(), CliError>;
}
