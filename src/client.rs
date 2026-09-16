use crate::dto::dry_run::DryRun;
use crate::dto::update::UpdateInput;
use crate::dto::{CreatedMonitor, Entitlements, Monitor, MonitorPage, Session, Workspace};
use crate::error::CliError;
use crate::model::{MonitorId, WorkspaceId};
use crate::monitoring::{CreateInput, DryRunInput, LifecycleInput};
use crate::wire::Response;
use async_trait::async_trait;

mod alerting;
mod incidents;
mod maintenance;
pub use alerting::AlertingApi;
pub use incidents::IncidentApi;
pub use maintenance::MaintenanceApi;
mod http;
mod memory;
pub use http::HttpFomkeeApi;
pub use memory::InMemoryFomkeeApi;

/// Public HTTP API port; success always carries the validated endpoint representation.
#[async_trait]
pub trait FomkeeApi: AlertingApi + IncidentApi + MaintenanceApi + Send + Sync {
    /// Resolve the token's current workspace identity.
    async fn session(&self) -> Result<Response<Session>, CliError>;
    /// Fetch public metadata for the authenticated workspace.
    async fn workspace(&self, workspace: &WorkspaceId) -> Result<Response<Workspace>, CliError>;
    /// Fetch API-owned plan limits without deriving local policy.
    async fn entitlements(
        &self,
        workspace: &WorkspaceId,
    ) -> Result<Response<Entitlements>, CliError>;
    /// Return exactly one page and an opaque continuation cursor.
    async fn list_monitors(
        &self,
        workspace: &WorkspaceId,
        limit: u32,
        after: Option<&str>,
    ) -> Result<Response<MonitorPage>, CliError>;
    /// Fetch a monitor detail representation.
    async fn get_monitor(
        &self,
        workspace: &WorkspaceId,
        monitor: &MonitorId,
    ) -> Result<Response<Monitor>, CliError>;
    /// Execute a non-persistent HTTP or Function trial.
    async fn dry_run(
        &self,
        workspace: &WorkspaceId,
        body: DryRunInput,
    ) -> Result<Response<DryRun>, CliError>;
    /// Create once; never replay an ambiguous mutation.
    async fn create_monitor(
        &self,
        workspace: &WorkspaceId,
        input: CreateInput,
    ) -> Result<Response<CreatedMonitor>, CliError>;
    /// Replace the editable monitor representation once.
    async fn update_monitor(
        &self,
        workspace: &WorkspaceId,
        monitor: &MonitorId,
        input: UpdateInput,
    ) -> Result<Response<Monitor>, CliError>;
    /// Apply one lifecycle operation and return the resulting monitor.
    async fn lifecycle(
        &self,
        workspace: &WorkspaceId,
        monitor: &MonitorId,
        input: LifecycleInput,
    ) -> Result<Response<Monitor>, CliError>;
    /// Delete once; callers must obtain confirmation beforehand.
    async fn delete_monitor(
        &self,
        workspace: &WorkspaceId,
        monitor: &MonitorId,
    ) -> Result<(), CliError>;
}

#[cfg(test)]
mod tests;
