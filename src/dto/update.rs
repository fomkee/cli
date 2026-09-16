use crate::dto::request::{Auth, Header};
use crate::error::CliError;
use crate::wire::{Response, Secret};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Full monitor replacement, validated without dropping explicit extension fields.
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct UpdateInput(Response<UpdateRequest>);

impl TryFrom<Value> for UpdateInput {
    type Error = CliError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Response::parse(value).map(Self).map_err(|_| CliError::InvalidInput(
            "update JSON must be a complete monitor replacement, including explicit auth actions and nullable fields".into()))
    }
}

#[derive(Clone, Deserialize, Serialize)]
struct UpdateRequest {
    name: String,
    #[serde(deserialize_with = "Option::deserialize")]
    description: Option<String>,
    tags: Vec<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    sla_target_parts_per_million: Option<u32>,
    #[serde(flatten)]
    config: UpdateConfig,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "config_type", rename_all = "snake_case")]
enum UpdateConfig {
    Http {
        #[serde(flatten)]
        active: ActiveUpdate,
        expected_status: String,
        #[serde(deserialize_with = "Option::deserialize")]
        response_time_max_ms: Option<u64>,
    },
    Function {
        #[serde(flatten)]
        active: ActiveUpdate,
        js_source: Secret<String>,
    },
    Heartbeat {
        schedule: UpdateSchedule,
        #[serde(deserialize_with = "Option::deserialize")]
        body_validation: Option<Secret<String>>,
    },
}

#[derive(Clone, Deserialize, Serialize)]
struct ActiveUpdate {
    url: String,
    method: String,
    timeout_secs: u64,
    interval_secs: u64,
    retry_max_attempts: u8,
    retry_base_backoff_ms: u64,
    retry_max_backoff_ms: u64,
    headers: Vec<Header>,
    auth: AuthChange,
    #[serde(deserialize_with = "Option::deserialize")]
    body: Option<Secret<RequestBody>>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum AuthChange {
    Preserve,
    Clear,
    Set { credentials: Auth },
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum UpdateSchedule {
    Interval {
        period_secs: u64,
        grace_secs: u64,
    },
    Cron {
        cron_expression: String,
        grace_secs: u64,
    },
}

#[derive(Clone, Deserialize, Serialize)]
struct RequestBody {
    format: BodyFormat,
    content: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum BodyFormat {
    Json,
    Xml,
    FormUrlencoded,
    Plain,
}
