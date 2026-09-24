use crate::config::is_hosted_api_url;
use crate::model::WorkspaceId;
use crate::workspace::Connection;

#[derive(Default)]
pub(crate) struct DisplayContext {
    pub(crate) workspace: Option<(WorkspaceId, String)>,
    pub(crate) hosted: bool,
}

impl DisplayContext {
    pub(crate) fn from_connection(connection: &Connection, workspace: WorkspaceId) -> Self {
        Self {
            workspace: connection
                .workspace_name()
                .map(|name| (workspace, name.to_owned())),
            hosted: is_hosted_api_url(&connection.config.base_url),
        }
    }
}
