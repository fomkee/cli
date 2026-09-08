use std::path::PathBuf;

use serde::{Deserialize, Serialize, Serializer};

use super::model::{CredentialBackend, WorkspaceAlias};
use crate::dto::Session;
use crate::model::WorkspaceId;
use crate::wire::Response;

/// Non-secret result of selecting or connecting a saved workspace.
#[derive(Serialize, Deserialize)]
pub struct ProfileOutput {
    pub alias: WorkspaceAlias,
    pub api_url: String,
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub slug: String,
    pub active: bool,
    pub credential_store: CredentialBackend,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_storage_notice: Option<String>,
}

/// A snapshot of locally connected workspaces.
#[derive(Serialize)]
pub struct WorkspaceList {
    pub active: Option<WorkspaceAlias>,
    pub workspaces: Vec<ProfileOutput>,
}

/// Receipt for local disconnection, not remote token revocation.
#[derive(Serialize, Deserialize)]
pub struct Disconnected {
    pub disconnected: WorkspaceAlias,
    pub active: Option<WorkspaceAlias>,
    pub api_key_revoked: bool,
}

/// The selected credential backend or ephemeral environment source.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialSource {
    Environment,
    Keyring,
    File,
}

impl CredentialSource {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Environment => "Environment",
            Self::Keyring => "Keyring",
            Self::File => "File",
        }
    }
}

impl From<CredentialBackend> for CredentialSource {
    fn from(value: CredentialBackend) -> Self {
        match value {
            CredentialBackend::Keyring => Self::Keyring,
            CredentialBackend::File => Self::File,
        }
    }
}

/// Non-secret information about the connection used by this invocation.
#[derive(Serialize)]
pub struct ConnectionInfo {
    pub alias: Option<WorkspaceAlias>,
    pub api_url: String,
    pub credential_source: CredentialSource,
}

/// The original verified session enriched with local connection metadata.
pub struct AuthStatus {
    pub session: Response<Session>,
    pub connection: ConnectionInfo,
}

impl Serialize for AuthStatus {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.session
            .serialize_with_field(serializer, "connection", &self.connection)
    }
}

/// Configuration locations resolved without opening files.
#[derive(Serialize)]
pub struct ConfigPaths {
    pub directory: PathBuf,
    pub workspaces: PathBuf,
    pub credentials: PathBuf,
}

/// Effective saved, environment, or unselected configuration.
#[derive(Serialize)]
#[serde(untagged)]
pub enum ConfigInfo {
    Saved {
        directory: PathBuf,
        alias: WorkspaceAlias,
        api_url: String,
        workspace_id: WorkspaceId,
        name: String,
        credential_source: CredentialBackend,
    },
    Environment {
        directory: PathBuf,
        api_url: String,
        credential_source: CredentialSource,
        alias: Option<WorkspaceAlias>,
    },
    Unselected {
        directory: PathBuf,
        connection: &'static str,
    },
}
