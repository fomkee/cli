use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use uuid::{Builder, Uuid};

use crate::error::CliError;
use crate::model::WorkspaceId;

/// Local name for a saved workspace connection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct WorkspaceAlias(String);

impl FromStr for WorkspaceAlias {
    type Err = CliError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.len() > 64
            || !value.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
            })
        {
            return Err(CliError::InvalidInput("workspace alias must contain 1–64 lowercase letters, digits, hyphens, or underscores".into()));
        }
        Ok(Self(value.to_owned()))
    }
}

impl TryFrom<String> for WorkspaceAlias {
    type Error = CliError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<WorkspaceAlias> for String {
    fn from(value: WorkspaceAlias) -> Self {
        value.0
    }
}

impl Display for WorkspaceAlias {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Opaque reference to a saved credential.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CredentialId(Uuid);

impl CredentialId {
    /// Generate an opaque reference using fallible operating-system randomness.
    pub fn generate() -> Result<Self, CliError> {
        let mut bytes = [0; 16];
        getrandom::fill(&mut bytes).map_err(|_| CliError::CredentialStore)?;
        Ok(Self(Builder::from_random_bytes(bytes).into_uuid()))
    }
}

impl Display for CredentialId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Backend that owns a saved token; reads never switch backends.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialBackend {
    #[default]
    Keyring,
    File,
}

/// Credential storage policy used only when connecting a workspace.
#[derive(Debug, Default, Clone, Copy, ValueEnum)]
pub enum CredentialPreference {
    #[default]
    Auto,
    Keyring,
    File,
}

/// Non-secret workspace metadata. The API origin and credential are selected together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceProfile {
    pub api_url: String,
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub credential_id: CredentialId,
    #[serde(default)]
    pub credential_store: CredentialBackend,
}

/// Local workspace registry; contains no API tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceRegistry {
    pub version: u32,
    pub active: Option<WorkspaceAlias>,
    pub workspaces: BTreeMap<WorkspaceAlias, WorkspaceProfile>,
}

impl Default for WorkspaceRegistry {
    fn default() -> Self {
        Self {
            version: 1,
            active: None,
            workspaces: BTreeMap::new(),
        }
    }
}
