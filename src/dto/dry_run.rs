use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::wire::Secret;

/// Execution verdict, distinct from monitor lifecycle.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Passed,
    Failed,
    ExecutionUnavailable,
    NotRun,
}

impl Status {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Passed => "Passed",
            Self::Failed => "Failed",
            Self::ExecutionUnavailable => "Execution unavailable",
            Self::NotRun => "Not run",
        }
    }
}

/// Public dry-run variants with typed execution and evidence fields.
#[derive(Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DryRun {
    Http(Http),
    Function {
        status: Status,
        http: Http,
        function: Function,
    },
}

impl DryRun {
    pub(crate) fn status(&self) -> Status {
        match self {
            Self::Http(http) => http.status,
            Self::Function { status, .. } => *status,
        }
    }
    pub(crate) fn http(&self) -> &Http {
        match self {
            Self::Http(http) | Self::Function { http, .. } => http,
        }
    }
}

/// HTTP execution evidence returned by either dry-run variant.
#[derive(Clone, Deserialize)]
pub struct Http {
    pub status: Status,
    pub summary: Option<String>,
    pub request: Option<Request>,
    pub response: Option<Capture>,
    pub validation: Option<Validation>,
    #[serde(default)]
    pub attempts: Vec<Attempt>,
    pub message: Option<String>,
}

/// Displayable request identity; sensitive request content stays in the wire envelope.
#[derive(Clone, Deserialize)]
pub struct Request {
    pub method: String,
    pub url: String,
}

#[derive(Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Capture {
    Http {
        status: u16,
        latency_ms: u64,
        body: Option<Body>,
    },
    ConnectionError {
        message: String,
        latency_ms: u64,
    },
    Timeout {
        elapsed_ms: u64,
        latency_ms: u64,
    },
}

/// Body capture metadata without captured content.
#[derive(Clone, Deserialize)]
pub struct Body {
    pub content_type: Option<String>,
    pub original_size: u64,
    pub stored_size: u64,
    pub truncated: bool,
}

/// The API's validation verdict and explanation.
#[derive(Clone, Deserialize)]
pub struct Validation {
    pub status: Status,
    pub message: Option<String>,
    pub reason: Option<String>,
}

/// One captured execution attempt.
#[derive(Clone, Deserialize)]
pub struct Attempt {
    pub attempt_number: u8,
    pub sent_at: String,
    pub completed_at: String,
    pub latency_ms: u64,
    pub response: Capture,
}

/// Function execution evidence and bounded API-provided console output.
#[derive(Clone, Deserialize)]
pub struct Function {
    pub status: Status,
    pub summary: Option<String>,
    pub reason: Option<String>,
    #[serde(default)]
    pub logs: Vec<ConsoleLine>,
    pub resource_usage: Option<Value>,
}

/// One console entry; diagnostic formatting cannot reveal its message.
#[derive(Clone, Deserialize)]
pub struct ConsoleLine {
    pub level: String,
    pub message: Secret<String>,
}
