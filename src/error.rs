use serde::Serialize;
use std::fmt::{self, Debug, Formatter};
use thiserror::Error;
pub mod response;
pub mod transport;
use response::ResponseFailure;
use transport::TransportFailure;

/// Typed failures with stable process exit codes and safe debug representations.
#[derive(Error)]
pub enum CliError {
    #[error(transparent)]
    Response(#[from] ResponseFailure),
    #[error(
        "connect a workspace with workspace connect, or provide FOMKEE_API_TOKEN through the environment"
    )]
    MissingCredentials,
    #[error(
        "credential store unavailable; unlock your OS keyring or use FOMKEE_API_TOKEN for this invocation"
    )]
    CredentialStore,
    #[error("saved credential is missing; disconnect and reconnect this workspace")]
    MissingSavedCredential,
    #[error("workspace configuration: {0}")]
    Configuration(String),
    #[error("skill export: {0}")]
    SkillExport(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("confirmation required: rerun destructive deletion with --yes")]
    ConfirmationRequired,
    #[error("API request failed ({status} {code}): {message}")]
    Api {
        status: u16,
        code: String,
        message: String,
        details: Option<serde_json::Value>,
        retry_after_secs: Option<u64>,
    },
    #[error(
        "request outcome is unknown; do not retry this mutation automatically; inspect current state"
    )]
    OutcomeUnknown(#[source] TransportFailure),
    #[error("transport failure: {0}")]
    Transport(#[source] TransportFailure),
    #[error("malformed API response: {0}")]
    MalformedResponse(String),
}
impl CliError {
    pub(crate) fn io(operation: &'static str, source: std::io::Error) -> Self {
        Self::Transport(TransportFailure::Io { operation, source })
    }
    /// Return the documented process exit category.
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Response(error) => error.exit_code(),
            Self::MissingCredentials | Self::CredentialStore | Self::MissingSavedCredential => 3,
            Self::Configuration(_) => 2,
            Self::InvalidInput(_) | Self::ConfirmationRequired => 2,
            Self::Api { status: 429, .. } => 4,
            Self::Api { .. } => 5,
            Self::OutcomeUnknown(_) => 6,
            Self::Transport(_) | Self::SkillExport(_) => 7,
            Self::MalformedResponse(_) => 8,
        }
    }
    /// Build the explicit machine-readable error envelope.
    pub fn as_json(&self) -> ErrorOutput {
        match self {
            Self::Response(error) => ErrorOutput {
                category: error.category(),
                status: Some(error.status),
                code: error.code(),
                message: error.to_string(),
                details: None,
                retry_after_secs: error.retry_after_secs,
            },
            Self::Api {
                status,
                code,
                message,
                details,
                retry_after_secs,
            } => ErrorOutput {
                category: "api",
                status: Some(*status),
                code: Some(code.clone()),
                message: message.clone(),
                details: details.clone(),
                retry_after_secs: *retry_after_secs,
            },
            Self::OutcomeUnknown(_) => ErrorOutput {
                category: "outcome_unknown",
                status: None,
                code: None,
                message: self.to_string(),
                details: None,
                retry_after_secs: None,
            },
            Self::MissingCredentials | Self::CredentialStore | Self::MissingSavedCredential => {
                self.simple_output("credentials")
            }
            Self::Configuration(_) => self.simple_output("configuration"),
            Self::SkillExport(_) => self.simple_output("filesystem"),
            Self::InvalidInput(_) => self.simple_output("validation"),
            Self::ConfirmationRequired => self.simple_output("confirmation_required"),
            Self::Transport(_) => self.simple_output("transport"),
            Self::MalformedResponse(_) => self.simple_output("protocol"),
        }
    }

    fn simple_output(&self, category: &'static str) -> ErrorOutput {
        ErrorOutput {
            category,
            status: None,
            code: None,
            message: self.to_string(),
            details: None,
            retry_after_secs: None,
        }
    }
}

impl Debug for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CliError(exit_code={}; details redacted)",
            self.exit_code()
        )
    }
}
/// Machine-readable error metadata; optional fields are omitted when unknown.
#[derive(Serialize)]
pub struct ErrorOutput {
    pub category: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_secs: Option<u64>,
}
