use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::model::{MonitorId, WorkspaceId};
use crate::wire::Secret;

pub mod dry_run;
pub mod request;

/// Workspace identity established by the API token.
#[derive(Clone, Deserialize)]
pub struct Session {
    pub workspace_id: WorkspaceId,
    pub caller_type: Option<String>,
    pub user_id: Option<String>,
    pub api_key_id: Option<String>,
    pub role: Option<String>,
    pub capabilities: Option<Value>,
}

/// Public workspace metadata; no membership lookup is implied.
#[derive(Clone, Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
}

/// Public monitor kind, shared by request and presentation decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MonitorKind {
    Http,
    Function,
    Heartbeat,
}

impl MonitorKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Http => "HTTP",
            Self::Function => "Function",
            Self::Heartbeat => "Heartbeat",
        }
    }
}

/// Monitoring lifecycle, deliberately distinct from observed health.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    Active,
    Paused,
    Disabled,
}

impl Lifecycle {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Paused => "Paused",
            Self::Disabled => "Disabled",
        }
    }
}

/// Common monitor response fields. Configuration is a tagged wire variant.
#[derive(Clone, Deserialize)]
pub struct Monitor {
    pub id: MonitorId,
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub state: Lifecycle,
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub sla_target_parts_per_million: Option<u64>,
    #[serde(flatten)]
    pub config: MonitorConfig,
}

/// Kind-specific response fields; the backend remains the policy authority.
#[derive(Clone, Deserialize)]
#[serde(tag = "config_type", rename_all = "snake_case")]
pub enum MonitorConfig {
    Http {
        #[serde(flatten)]
        active: ActiveConfig,
        expected_status: Option<String>,
        response_time_max_ms: Option<u64>,
    },
    Function {
        #[serde(flatten)]
        active: ActiveConfig,
        function_id: Option<String>,
        js_source: Option<Secret<String>>,
    },
    Heartbeat {
        #[serde(flatten)]
        schedule: Schedule,
        body_validation: Option<Secret<Value>>,
    },
}

impl MonitorConfig {
    pub(crate) fn kind(&self) -> MonitorKind {
        match self {
            Self::Http { .. } => MonitorKind::Http,
            Self::Function { .. } => MonitorKind::Function,
            Self::Heartbeat { .. } => MonitorKind::Heartbeat,
        }
    }
    pub(crate) fn active(&self) -> Option<&ActiveConfig> {
        match self {
            Self::Http { active, .. } | Self::Function { active, .. } => Some(active),
            Self::Heartbeat { .. } => None,
        }
    }
}

/// Active-monitor response settings without locally invented defaults.
#[derive(Clone, Deserialize)]
pub struct ActiveConfig {
    pub url: String,
    pub method: String,
    pub interval_secs: u64,
    pub timeout_secs: Option<u64>,
    pub retry_max_attempts: Option<u64>,
    pub retry_base_backoff_ms: Option<u64>,
    pub retry_max_backoff_ms: Option<u64>,
    pub auth: Option<AuthStatus>,
    #[serde(default)]
    pub headers: Vec<request::Header>,
    pub body: Option<Secret<Value>>,
}

/// Heartbeat scheduling representation supplied by the API.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "schedule_type", rename_all = "snake_case")]
pub enum Schedule {
    Interval {
        period_secs: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        grace_secs: Option<u64>,
    },
    Cron {
        cron_expression: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        grace_secs: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timezone: Option<String>,
    },
}

/// The detail endpoint reports credential kind, never credential values.
#[derive(Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthStatus {
    None,
    Basic,
    Bearer,
    ApiKey { header_name: String },
}

impl AuthStatus {
    pub(crate) fn configured_label(&self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Basic => Some("Basic"),
            Self::Bearer => Some("Bearer"),
            Self::ApiKey { .. } => Some("API key"),
        }
    }
}

/// One explicit inventory page.
#[derive(Clone, Deserialize)]
pub struct MonitorPage {
    pub items: Vec<Monitor>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub next_cursor: Option<String>,
}

/// Creation result, including an optional intentionally one-time secret.
#[derive(Clone, Deserialize)]
pub struct CreatedMonitor {
    pub monitor: Monitor,
    pub heartbeat_secret: Option<Secret<String>>,
}

/// Extensible entitlement response with a typed path to API-owned defaults.
#[derive(Clone, Deserialize)]
pub struct Entitlements {
    pub plan: Plan,
    pub grant_standing: Option<Value>,
    #[serde(flatten)]
    pub metadata: BTreeMap<String, Value>,
}

/// Assigned plan identity and its current revision.
#[derive(Clone, Deserialize)]
pub struct Plan {
    pub display_name: Option<String>,
    pub code: Option<String>,
    pub status: Option<String>,
    pub revision: PlanRevision,
    #[serde(flatten)]
    pub metadata: BTreeMap<String, Value>,
}

/// API-owned capability limits and extensible revision metadata.
#[derive(Clone, Deserialize)]
pub struct PlanRevision {
    pub entitlements: Limits,
    #[serde(flatten)]
    pub metadata: BTreeMap<String, Value>,
}

/// Typed monitoring defaults with extensible capability sections.
#[derive(Clone, Deserialize, Serialize)]
pub struct Limits {
    pub monitoring: MonitoringLimits,
    #[serde(flatten)]
    pub sections: BTreeMap<String, Value>,
}

/// Monitoring limits; absent defaults are never invented by the CLI.
#[derive(Clone, Deserialize, Serialize)]
pub struct MonitoringLimits {
    pub minimum_check_interval_seconds: Option<u64>,
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}

pub mod update;

pub mod alerting;
pub mod incidents;
pub mod maintenance;
