use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{MonitorKind, Schedule};
use crate::wire::Secret;

/// Creation wire data; missing values remain for the API to validate.
#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct CreateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(flatten)]
    pub config: CreateConfig,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "config_type", rename_all = "snake_case")]
pub(crate) enum CreateConfig {
    Http {
        #[serde(flatten)]
        active: ActiveRequest,
        #[serde(skip_serializing_if = "Option::is_none")]
        expected_status: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        response_time_max_ms: Option<u64>,
    },
    Function {
        #[serde(flatten)]
        active: ActiveRequest,
        #[serde(skip_serializing_if = "Option::is_none")]
        js_source: Option<Secret<String>>,
    },
    Heartbeat {
        #[serde(flatten)]
        schedule: Schedule,
        #[serde(skip_serializing_if = "Option::is_none")]
        js_source: Option<Secret<String>>,
    },
}

impl CreateConfig {
    pub(crate) fn kind(&self) -> MonitorKind {
        match self {
            Self::Http { .. } => MonitorKind::Http,
            Self::Function { .. } => MonitorKind::Function,
            Self::Heartbeat { .. } => MonitorKind::Heartbeat,
        }
    }
    pub(crate) fn set_interval(&mut self, seconds: u64) {
        match self {
            Self::Http { active, .. } | Self::Function { active, .. } => {
                active.interval_secs = Some(seconds)
            }
            Self::Heartbeat { .. } => {}
        }
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub(crate) struct ActiveRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_max_attempts: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_base_backoff_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_max_backoff_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<Header>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Secret<Value>>,
}

/// A request header with explicitly secret-bearing content.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Header {
    pub name: String,
    pub value: Secret<String>,
}

/// Write-only credentials accepted by the public HTTP request schema.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Auth {
    Basic {
        username: Secret<String>,
        password: Secret<String>,
    },
    Bearer {
        token: Secret<String>,
    },
    ApiKey {
        header_name: String,
        value: Secret<String>,
    },
}

/// Pause fields serialized exactly as the public lifecycle request.
#[derive(Debug, Clone, Serialize)]
pub struct PauseRequest {
    pub reason: Option<Secret<String>>,
    pub resume_at: Option<String>,
}

/// Disable fields serialized exactly as the public lifecycle request.
#[derive(Debug, Clone, Serialize)]
pub struct DisableRequest {
    pub reason: Option<Secret<String>>,
}
