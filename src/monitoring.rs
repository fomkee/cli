use crate::dto::request::{CreateRequest, DisableRequest, PauseRequest};
use crate::dto::{Entitlements, MonitorKind};
use crate::duration;
use crate::error::CliError;
use crate::wire::{Response, Secret};
use clap::{Args, Subcommand, ValueHint};
use serde::Serialize;
use serde_json::Value;
use std::fmt::{self, Debug, Formatter};
use std::fs;
use std::io::{self, Read};

#[derive(Debug, Subcommand)]
pub(crate) enum CreateCommand {
    Http(HttpCreateArgs),
    Function(FunctionCreateArgs),
    Heartbeat(HeartbeatCreateArgs),
}

#[derive(Debug, Args)]
pub(crate) struct CommonCreateArgs {
    /// Display name; defaults to hostname/path for HTTP and Function monitors.
    #[arg(long, conflicts_with = "file")]
    name: Option<String>,
    #[arg(long, conflicts_with = "file")]
    description: Option<String>,
    #[arg(long, value_delimiter = ',', conflicts_with = "file")]
    tags: Vec<String>,
    /// Complete public API request JSON; use - for standard input.
    #[arg(long, value_hint = ValueHint::FilePath)]
    file: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct HttpCreateArgs {
    #[command(flatten)]
    common: CommonCreateArgs,
    /// Target URL. Supply --file instead for a complete API request.
    #[arg(required_unless_present = "file", conflicts_with = "file", value_hint = ValueHint::Url)]
    url: Option<String>,
    /// Check interval, e.g. 30s or 5m. Defaults to the API's workspace minimum.
    #[arg(long = "interval", value_name = "DURATION", value_parser = duration::seconds, conflicts_with = "file")]
    interval_secs: Option<u64>,
    /// HTTP method; omitted by default so the API chooses.
    #[arg(long, conflicts_with = "file")]
    method: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct FunctionCreateArgs {
    #[command(flatten)]
    common: CommonCreateArgs,
    #[arg(required_unless_present = "file", conflicts_with = "file", value_hint = ValueHint::Url)]
    url: Option<String>,
    /// Check interval, e.g. 30s or 5m. Defaults to the API's workspace minimum.
    #[arg(long = "interval", value_name = "DURATION", value_parser = duration::seconds, conflicts_with = "file")]
    interval_secs: Option<u64>,
    /// JavaScript source file; use - for standard input.
    #[arg(long = "script", value_name = "FILE", required_unless_present = "file", conflicts_with = "file", value_hint = ValueHint::FilePath)]
    js_source_file: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct HeartbeatCreateArgs {
    #[command(flatten)]
    common: CommonCreateArgs,
    /// Expected heartbeat period, e.g. 24h; required unless --cron or --file is supplied.
    #[arg(long = "every", value_name = "DURATION", value_parser = duration::seconds, conflicts_with_all = ["cron_expression", "file"], required_unless_present_any = ["cron_expression", "file"])]
    period_secs: Option<u64>,
    #[arg(long = "cron", conflicts_with_all = ["period_secs", "file"])]
    cron_expression: Option<String>,
    #[arg(long = "grace", value_name = "DURATION", value_parser = duration::seconds, conflicts_with = "file")]
    grace_secs: Option<u64>,
    /// Optional heartbeat body-validation JavaScript source file.
    #[arg(long = "script", value_name = "FILE", conflicts_with = "file", value_hint = ValueHint::FilePath)]
    js_source_file: Option<String>,
}

mod builders;
pub(crate) mod execute;

fn required<T>(value: Option<T>, flag: &str) -> Result<T, CliError> {
    value.ok_or_else(|| CliError::InvalidInput(format!("{flag} is required when --file is absent")))
}

fn read_text_input(path: &str) -> Result<String, CliError> {
    if path == "-" {
        let mut text = String::new();
        io::stdin()
            .lock()
            .read_to_string(&mut text)
            .map_err(|_| CliError::InvalidInput("cannot read standard input".into()))?;
        return Ok(text);
    }
    fs::read_to_string(path).map_err(|_| CliError::InvalidInput(format!("cannot read {path}")))
}

/// Private, typed creation request with a redacted diagnostic representation.
#[derive(Clone, Serialize)]
#[serde(transparent)]
pub struct CreateInput(InputSource);

#[derive(Clone, Serialize)]
#[serde(untagged)]
enum InputSource {
    Generated(CreateRequest),
    Explicit(Response<CreateRequest>),
}

impl Debug for CreateInput {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("CreateInput([REDACTED])")
    }
}

impl CreateInput {
    pub(crate) fn kind(&self) -> MonitorKind {
        match &self.0 {
            InputSource::Generated(request) => request.config.kind(),
            InputSource::Explicit(request) => request.data().config.kind(),
        }
    }

    /// Fill a generated request from the API's minimum; never rewrite explicit file input.
    pub fn use_api_interval(&mut self, entitlements: &Entitlements) -> Result<(), CliError> {
        let request = match &mut self.0 {
            InputSource::Generated(request) => request,
            InputSource::Explicit(_) => {
                return Err(CliError::InvalidInput(
                    "a complete --file request is never overridden by convenience defaults".into(),
                ));
            }
        };
        let seconds = entitlements.plan.revision.entitlements.monitoring.minimum_check_interval_seconds
            .filter(|seconds| *seconds > 0)
            .ok_or_else(|| CliError::MalformedResponse("entitlement response has no valid minimum check interval; supply --interval explicitly".into()))?;
        request.config.set_interval(seconds);
        Ok(())
    }
}

impl TryFrom<Value> for CreateInput {
    type Error = CliError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Response::parse(value)
            .map(|request| Self(InputSource::Explicit(request)))
            .map_err(|_| {
                CliError::InvalidInput(
                "creation JSON must match the public http, function, or heartbeat request shape"
                    .into(),
            )
            })
    }
}

/// Non-persistent dry-run input; heartbeat has no such API operation.
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct DryRunInput(CreateInput);

impl TryFrom<Value> for DryRunInput {
    type Error = CliError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let input = CreateInput::try_from(value)?;
        match input.kind() {
            MonitorKind::Http | MonitorKind::Function => Ok(Self(input)),
            MonitorKind::Heartbeat => Err(CliError::InvalidInput(
                "Heartbeat monitors have no unsaved dry-run API".into(),
            )),
        }
    }
}

/// Typed lifecycle requests; source content stays redacted.
#[derive(Debug, Clone)]
pub enum LifecycleInput {
    Pause(PauseRequest),
    Resume,
    Disable(DisableRequest),
    Enable,
}

impl LifecycleInput {
    /// Pause with optional API-interpreted reason and resume timestamp.
    pub fn pause(reason: Option<String>, resume_at: Option<String>) -> Self {
        Self::Pause(PauseRequest {
            reason: reason.map(Secret::new),
            resume_at,
        })
    }
    /// Resume a paused monitor.
    pub fn resume() -> Self {
        Self::Resume
    }
    /// Disable with an optional API-interpreted reason.
    pub fn disable(reason: Option<String>) -> Self {
        Self::Disable(DisableRequest {
            reason: reason.map(Secret::new),
        })
    }
    /// Enable a disabled monitor.
    pub fn enable() -> Self {
        Self::Enable
    }
}

pub(crate) fn read_json_input(path: &str) -> Result<Value, CliError> {
    serde_json::from_str(&read_text_input(path)?)
        .map_err(|_| CliError::InvalidInput("input is not valid JSON".into()))
}

#[cfg(test)]
mod tests;
