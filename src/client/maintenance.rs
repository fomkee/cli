use crate::dto::alerting::Page;
use crate::dto::maintenance::{CreateMaintenanceInput, Maintenance};
use crate::error::CliError;
use crate::model::{MaintenanceId, WorkspaceId};
use crate::wire::Response;
use async_trait::async_trait;

/// Public maintenance operations.
#[async_trait]
pub trait MaintenanceApi: Send + Sync {
    /// Return one maintenance page.
    async fn list_maintenance(
        &self,
        workspace: &WorkspaceId,
        limit: u32,
        after: Option<&str>,
    ) -> Result<Response<Page<Maintenance>>, CliError>;
    /// Schedule one maintenance window without retrying.
    async fn create_maintenance(
        &self,
        workspace: &WorkspaceId,
        input: CreateMaintenanceInput,
    ) -> Result<Response<Maintenance>, CliError>;
    /// Cancel one maintenance window without retrying.
    async fn cancel_maintenance(
        &self,
        workspace: &WorkspaceId,
        id: &MaintenanceId,
    ) -> Result<Response<Maintenance>, CliError>;
}
