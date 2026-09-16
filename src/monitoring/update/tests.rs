use super::*;
use crate::commands::{Cli, Command, MonitorCommand};
use clap::Parser;

const ID: &str = "00000000-0000-0000-0000-000000000001";
fn changes(flags: &[&str]) -> UpdateArgs {
    parsed_changes(flags).unwrap()
}
fn parsed_changes(flags: &[&str]) -> Option<UpdateArgs> {
    let args = ["fomkeecli", "monitor", "update", ID]
        .into_iter()
        .chain(flags.iter().copied());
    let Command::Monitor {
        command: MonitorCommand::Update { changes, .. },
    } = Cli::try_parse_from(args).unwrap().command
    else {
        return None;
    };
    Some(*changes)
}

fn http() -> Value {
    json!({"id": ID, "workspace_id": ID, "state": "active", "config_type": "http", "name": "Original", "description": "keep", "tags": ["prod"], "sla_target_parts_per_million": 999000,
        "url": "https://example.com/health", "method": "POST", "timeout_secs": 10, "interval_secs": 300,
        "retry_max_attempts": 3, "retry_base_backoff_ms": 1000, "retry_max_backoff_ms": 30000,
        "headers": [{"name": "X-Key", "value": "private-key"}], "auth": {"type": "bearer"}, "body": {"format": "plain", "content": "private-body"}, "expected_status": "any_success", "response_time_max_ms": null})
}
fn update(value: Value, flags: &[&str]) -> Value {
    serde_json::to_value(from_current(&Response::parse(value).unwrap(), changes(flags)).unwrap())
        .unwrap()
}
#[test]
fn test_name_update_preserves_credentials_headers_body_and_other_settings() {
    // Act
    let result = update(http(), &["--name", "Renamed"]);
    // Assert
    assert_preserved_http(&result);
}
fn assert_preserved_http(value: &Value) {
    assert_eq!(value.get("name").unwrap(), "Renamed");
    assert_eq!(value.get("auth").unwrap(), &json!({"action": "preserve"}));
    assert_eq!(
        value.get("headers").unwrap(),
        http().get("headers").unwrap()
    );
    assert_eq!(value.get("body").unwrap(), http().get("body").unwrap());
    assert_eq!(value.get("sla_target_parts_per_million").unwrap(), 999000);
    assert_eq!(value.get("interval_secs").unwrap(), 300);
    assert_eq!(value.get("tags").unwrap(), &json!(["prod"]));
    assert!(value.get("state").is_none() && value.get("id").is_none());
}
#[test]
fn test_updates_active_request_settings_with_friendly_durations() {
    let result = update(
        http(),
        &["--interval", "5m", "--timeout", "20s", "--method", "GET"],
    );
    assert_eq!(
        (
            result.get("interval_secs").unwrap().as_u64(),
            result.get("timeout_secs").unwrap().as_u64(),
            result.get("method").unwrap().as_str()
        ),
        (Some(300), Some(20), Some("GET"))
    );
}
#[test]
fn test_empty_description_and_tags_explicitly_clear_values() {
    let result = update(http(), &["--description", "", "--tags", ""]);
    assert_eq!(
        (
            result.get("description").unwrap().clone(),
            result.get("tags").unwrap().clone()
        ),
        (Value::Null, json!([]))
    );
}
#[test]
fn test_missing_editable_detail_field_prevents_replacement() {
    let mut detail = http();
    detail.as_object_mut().unwrap().remove("headers");
    let result = from_current(
        &Response::parse(detail).unwrap(),
        changes(&["--name", "New"]),
    );
    assert!(matches!(result, Err(CliError::MalformedResponse(_))));
}
#[test]
fn test_rejects_heartbeat_flags_on_http_monitor() {
    let result = from_current(
        &Response::parse(http()).unwrap(),
        changes(&["--every", "1h"]),
    );
    assert!(matches!(result, Err(CliError::InvalidInput(_))));
}
#[test]
fn test_preserves_function_source_exactly() {
    let mut detail = http();
    detail
        .as_object_mut()
        .unwrap()
        .insert("config_type".into(), json!("function"));
    detail
        .as_object_mut()
        .unwrap()
        .insert("js_source".into(), json!("  throw new Error('keep');\n"));
    let result = update(detail, &["--name", "New"]);
    assert_eq!(
        result.get("js_source").unwrap(),
        "  throw new Error('keep');\n"
    );
}
fn heartbeat() -> Value {
    json!({"id": ID, "workspace_id": ID, "state": "active", "config_type": "heartbeat", "name": "Cron", "description": null, "tags": [], "sla_target_parts_per_million": null,
        "schedule_type": "interval", "period_secs": 3600, "grace_secs": 60, "body_validation": {"js_source": "console.log('unchanged');"}})
}
#[test]
fn test_heartbeat_update_preserves_validation_and_converts_schedule_shape() {
    let result = update(heartbeat(), &["--cron", "0 * * * *"]);
    assert_eq!(
        result.get("schedule").unwrap(),
        &json!({"type": "cron", "cron_expression": "0 * * * *", "grace_secs": 60})
    );
    assert_eq!(
        result.get("body_validation").unwrap(),
        "console.log('unchanged');"
    );
}
#[test]
fn test_null_heartbeat_validation_stays_null() {
    let mut detail = heartbeat();
    detail
        .as_object_mut()
        .unwrap()
        .insert("body_validation".into(), Value::Null);
    let result = update(detail, &["--grace", "2m"]);
    assert_eq!(result.get("body_validation").unwrap(), &Value::Null);
    assert_eq!(
        result.get("schedule").unwrap().get("grace_secs").unwrap(),
        120
    );
}
#[test]
fn test_requires_at_least_one_update_option() {
    assert!(Cli::try_parse_from(["fomkeecli", "monitor", "update", ID]).is_err());
}
#[test]
fn test_complete_replacement_requires_explicit_nullable_fields() {
    let mut body = update(http(), &["--name", "New"]);
    body.as_object_mut().unwrap().remove("body");
    assert!(UpdateInput::try_from(body).is_err());
}
#[test]
fn test_complete_replacement_preserves_extension_fields_and_redacts_debug() {
    let mut body = update(http(), &["--name", "New"]);
    body.as_object_mut()
        .unwrap()
        .insert("future".into(), json!("retained"));
    let input = UpdateInput::try_from(body.clone()).unwrap();
    assert_eq!(serde_json::to_value(&input).unwrap(), body);
    assert!(!format!("{input:?}").contains("private-key"));
}
