use crate::dto::dry_run::DryRun;
use crate::dto::update::UpdateInput;
use crate::dto::{CreatedMonitor, Entitlements, Monitor, MonitorPage, Session, Workspace};
use crate::error::CliError;
use crate::model::{MonitorId, WorkspaceId};
use crate::monitoring::{CreateInput, DryRunInput, LifecycleInput};
use crate::wire::Response;
use async_trait::async_trait;

use super::FomkeeApi;
use serde_json::json;
/// Isolated API port with valid identity and explicitly unconfigured operations.
pub struct InMemoryFomkeeApi {
    workspace: WorkspaceId,
}
impl InMemoryFomkeeApi {
    /// Configure the workspace identity used by session and workspace discovery.
    pub fn with_session(workspace: WorkspaceId) -> Self {
        Self { workspace }
    }
}

fn unconfigured<T>() -> Result<T, CliError> {
    Err(CliError::Configuration(
        "in-memory API operation has no configured response".into(),
    ))
}

#[async_trait]
impl FomkeeApi for InMemoryFomkeeApi {
    async fn session(&self) -> Result<Response<Session>, CliError> {
        Response::parse(json!({"workspace_id": self.workspace,"role":"api_key"}))
    }
    async fn workspace(&self, workspace: &WorkspaceId) -> Result<Response<Workspace>, CliError> {
        if *workspace != self.workspace {
            return unconfigured();
        }
        Response::parse(json!({"id":workspace,"name":"Test workspace","slug":"test-workspace"}))
    }
    async fn entitlements(&self, _: &WorkspaceId) -> Result<Response<Entitlements>, CliError> {
        unconfigured()
    }
    async fn list_monitors(
        &self,
        _: &WorkspaceId,
        _: u32,
        _: Option<&str>,
    ) -> Result<Response<MonitorPage>, CliError> {
        Response::parse(json!({"items":[],"next_cursor":null}))
    }
    async fn get_monitor(
        &self,
        _: &WorkspaceId,
        _: &MonitorId,
    ) -> Result<Response<Monitor>, CliError> {
        unconfigured()
    }
    async fn dry_run(&self, _: &WorkspaceId, _: DryRunInput) -> Result<Response<DryRun>, CliError> {
        unconfigured()
    }
    async fn create_monitor(
        &self,
        _: &WorkspaceId,
        _: CreateInput,
    ) -> Result<Response<CreatedMonitor>, CliError> {
        unconfigured()
    }
    async fn update_monitor(
        &self,
        _: &WorkspaceId,
        _: &MonitorId,
        _: UpdateInput,
    ) -> Result<Response<Monitor>, CliError> {
        unconfigured()
    }
    async fn lifecycle(
        &self,
        _: &WorkspaceId,
        _: &MonitorId,
        _: LifecycleInput,
    ) -> Result<Response<Monitor>, CliError> {
        unconfigured()
    }
    async fn delete_monitor(&self, _: &WorkspaceId, _: &MonitorId) -> Result<(), CliError> {
        unconfigured()
    }
}

mod alerting;
mod incidents;
mod maintenance;
