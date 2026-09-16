use super::{read_json_input, read_text_input};
use crate::client::FomkeeApi;
use crate::dto::update::UpdateInput;
use crate::dto::{Monitor, MonitorConfig, Schedule};
use crate::duration;
use crate::error::CliError;
use crate::model::{MonitorId, WorkspaceId};
use crate::wire::Response;
use clap::Args;
use serde_json::{Map, Value, json};

#[derive(Debug, Args)]
#[group(required = true, multiple = true)]
pub(crate) struct UpdateArgs {
    /// Complete replacement settings in JSON; use - for standard input.
    #[arg(long, conflicts_with_all = ["name", "description", "tags", "url", "interval", "method", "timeout", "script", "every", "cron", "grace", "expected_status"])]
    file: Option<String>,
    /// New name for this monitor.
    #[arg(long)]
    name: Option<String>,
    /// New description; use an empty string to clear it.
    #[arg(long)]
    description: Option<String>,
    /// Replace tags with a comma-separated list; use an empty string to clear them.
    #[arg(long, value_delimiter = ',')]
    tags: Option<Vec<String>>,
    /// New website or endpoint URL to check.
    #[arg(long)]
    url: Option<String>,
    /// Time between checks, e.g. 30s or 5m.
    #[arg(long, value_parser = duration::seconds)]
    interval: Option<u64>,
    /// HTTP method to use, e.g. GET or POST.
    #[arg(long)]
    method: Option<String>,
    /// Maximum wait for a response, e.g. 10s.
    #[arg(long, value_parser = duration::seconds)]
    timeout: Option<u64>,
    /// JavaScript file for a Function check or Heartbeat body validation.
    #[arg(long)]
    script: Option<String>,
    /// Expected time between Heartbeat pings, e.g. 1h.
    #[arg(long, value_parser = duration::seconds, conflicts_with = "cron")]
    every: Option<u64>,
    /// Expected Heartbeat schedule, e.g. "0 * * * *".
    #[arg(long)]
    cron: Option<String>,
    /// Extra time to allow a late Heartbeat ping, e.g. 5m.
    #[arg(long, value_parser = duration::seconds)]
    grace: Option<u64>,
    /// Expected HTTP status, e.g. any_success, exact:200, or range:200-299.
    #[arg(long)]
    expected_status: Option<String>,
}

pub(crate) async fn execute(
    api: &impl FomkeeApi,
    workspace: &WorkspaceId,
    monitor: &MonitorId,
    args: UpdateArgs,
) -> Result<Response<Monitor>, CliError> {
    let input = match &args.file {
        Some(file) => UpdateInput::try_from(read_json_input(file)?)?,
        None => from_current(&api.get_monitor(workspace, monitor).await?, args)?,
    };
    api.update_monitor(workspace, monitor, input).await
}

fn from_current(
    current: &Response<Monitor>,
    mut args: UpdateArgs,
) -> Result<UpdateInput, CliError> {
    let mut body = editable(current)?;
    for (key, value) in [
        ("name", args.name.take()),
        ("description", args.description.take()),
    ] {
        if let Some(value) = value {
            body.insert(
                key.into(),
                if key == "description" && value.is_empty() {
                    Value::Null
                } else {
                    json!(value)
                },
            );
        }
    }
    if let Some(tags) = args.tags.take() {
        body.insert(
            "tags".into(),
            json!(
                tags.into_iter()
                    .filter(|tag| !tag.is_empty())
                    .collect::<Vec<_>>()
            ),
        );
    }
    match &current.data().config {
        MonitorConfig::Http { .. } | MonitorConfig::Function { .. } => {
            apply_active(&mut body, args)?
        }
        MonitorConfig::Heartbeat { .. } => apply_heartbeat(&mut body, args)?,
    }
    UpdateInput::try_from(Value::Object(body))
}

fn editable(current: &Response<Monitor>) -> Result<Map<String, Value>, CliError> {
    let raw = serde_json::to_value(current)
        .map_err(|_| CliError::InvalidInput("cannot encode monitor".into()))?;
    let mut body = Map::new();
    copy(
        &raw,
        &mut body,
        &[
            "config_type",
            "name",
            "description",
            "tags",
            "sla_target_parts_per_million",
        ],
    )?;
    match &current.data().config {
        MonitorConfig::Http { .. } => {
            active(&raw, &mut body)?;
            copy(
                &raw,
                &mut body,
                &["expected_status", "response_time_max_ms"],
            )?;
        }
        MonitorConfig::Function { .. } => {
            active(&raw, &mut body)?;
            copy(&raw, &mut body, &["js_source"])?;
        }
        MonitorConfig::Heartbeat { schedule, .. } => heartbeat(&raw, &mut body, schedule)?,
    }
    Ok(body)
}

fn heartbeat(
    raw: &Value,
    body: &mut Map<String, Value>,
    schedule: &Schedule,
) -> Result<(), CliError> {
    let kind = match schedule {
        Schedule::Interval { .. } => "interval",
        Schedule::Cron { .. } => "cron",
    };
    let mut schedule_body = Map::new();
    let fields: &[&str] = match schedule {
        Schedule::Interval { .. } => &["period_secs", "grace_secs"],
        Schedule::Cron { .. } => &["cron_expression", "grace_secs"],
    };
    copy(raw, &mut schedule_body, fields)?;
    schedule_body.insert("type".into(), json!(kind));
    body.insert("schedule".into(), Value::Object(schedule_body));
    let validation = raw.get("body_validation").ok_or_else(incomplete)?;
    let source = if validation.is_null() {
        Value::Null
    } else {
        validation
            .get("js_source")
            .filter(|v| v.is_string())
            .cloned()
            .ok_or_else(incomplete)?
    };
    body.insert("body_validation".into(), source);
    Ok(())
}

fn incomplete() -> CliError {
    CliError::MalformedResponse("monitor detail lacks editable settings; no update sent".into())
}
fn copy(raw: &Value, body: &mut Map<String, Value>, fields: &[&str]) -> Result<(), CliError> {
    for field in fields {
        body.insert(
            (*field).into(),
            raw.get(*field).cloned().ok_or_else(incomplete)?,
        );
    }
    Ok(())
}
fn active(raw: &Value, body: &mut Map<String, Value>) -> Result<(), CliError> {
    copy(
        raw,
        body,
        &[
            "url",
            "method",
            "interval_secs",
            "timeout_secs",
            "retry_max_attempts",
            "retry_base_backoff_ms",
            "retry_max_backoff_ms",
            "headers",
            "body",
        ],
    )?;
    body.insert("auth".into(), json!({"action": "preserve"}));
    Ok(())
}
fn apply_active(body: &mut Map<String, Value>, args: UpdateArgs) -> Result<(), CliError> {
    if args.every.is_some() || args.cron.is_some() || args.grace.is_some() {
        return Err(CliError::InvalidInput(
            "--every, --cron and --grace require a Heartbeat monitor".into(),
        ));
    }
    for (key, value) in [
        ("url", args.url),
        ("method", args.method),
        ("expected_status", args.expected_status),
    ] {
        if let Some(value) = value {
            if key == "expected_status" && body.contains_key("js_source") {
                return Err(CliError::InvalidInput(
                    "--expected-status requires an HTTP monitor".into(),
                ));
            }
            body.insert(key.into(), json!(value));
        }
    }
    for (key, value) in [
        ("interval_secs", args.interval),
        ("timeout_secs", args.timeout),
    ] {
        if let Some(value) = value {
            body.insert(key.into(), json!(value));
        }
    }
    if let Some(script) = args.script {
        if !body.contains_key("js_source") {
            return Err(CliError::InvalidInput(
                "--script requires a Function or Heartbeat monitor".into(),
            ));
        }
        body.insert("js_source".into(), json!(read_text_input(&script)?));
    }
    Ok(())
}
fn apply_heartbeat(body: &mut Map<String, Value>, args: UpdateArgs) -> Result<(), CliError> {
    if args.url.is_some()
        || args.method.is_some()
        || args.interval.is_some()
        || args.timeout.is_some()
        || args.expected_status.is_some()
    {
        return Err(CliError::InvalidInput(
            "active HTTP settings cannot update a Heartbeat monitor".into(),
        ));
    }
    let schedule = body
        .get_mut("schedule")
        .and_then(Value::as_object_mut)
        .ok_or_else(incomplete)?;
    if let Some(period) = args.every {
        schedule.remove("cron_expression");
        schedule.insert("type".into(), json!("interval"));
        schedule.insert("period_secs".into(), json!(period));
    }
    if let Some(cron) = args.cron {
        schedule.remove("period_secs");
        schedule.insert("type".into(), json!("cron"));
        schedule.insert("cron_expression".into(), json!(cron));
    }
    if let Some(grace) = args.grace {
        schedule.insert("grace_secs".into(), json!(grace));
    }
    if let Some(script) = args.script {
        body.insert("body_validation".into(), json!(read_text_input(&script)?));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
