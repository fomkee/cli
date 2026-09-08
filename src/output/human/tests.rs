use super::fixtures::{self, Fixture};
use super::*;
use crate::dto::Monitor;
use crate::wire::Response;
use serde_json::{Value, json};
use unicode_width::UnicodeWidthStr;

fn render(value: &Value, view: Fixture, details: bool, width: u16, color: bool) -> String {
    fixtures::render(
        value,
        view,
        details,
        width,
        color,
        &DisplayContext::default(),
    )
}

fn a_monitor() -> Value {
    json!({
        "id":"361aca90-733d-4b11-a398-ad961bfd7e48", "name":"Rbasovo",
        "config_type":"http", "state":"active", "url":"https://example.com/health",
        "method":"GET", "interval_secs":300, "timeout_secs":10,
        "expected_status":"any_success", "description":null, "tags":[],
        "auth":{"type":"none"}, "headers":[], "body":null,
        "retry_max_attempts":3, "retry_base_backoff_ms":1000, "retry_max_backoff_ms":30000,
        "created_at":"2026-09-07T19:17:31.793900Z", "created_by":"creator-id",
        "updated_at":"2026-09-07T19:17:31.793900Z", "workspace_id":"00000000-0000-0000-0000-000000000001"
    })
}

fn plain(value: &Value, view: Fixture, details: bool, width: u16) -> String {
    render(value, view, details, width, false)
}

#[test]
fn test_monitor_summary_matches_design_snapshot() {
    // Arrange
    let monitor = a_monitor();

    // Act
    let output = plain(&monitor, Fixture::MonitorGet, false, 120);

    // Assert
    assert_eq!(
        output.trim_end(),
        include_str!("snapshots/monitor.txt").trim_end()
    );
}

#[test]
fn test_monitor_details_match_grouped_snapshot() {
    // Arrange
    let monitor = a_monitor();

    // Act
    let output = plain(&monitor, Fixture::MonitorGet, true, 120);

    // Assert
    assert_eq!(
        output.trim_end(),
        include_str!("snapshots/monitor-details.txt").trim_end()
    );
}

#[test]
fn test_narrow_inventory_uses_records_and_preserves_ids() {
    // Arrange
    let inventory = json!({"items":[a_monitor()], "next_cursor":"opaque-cursor"});

    // Act
    let output = plain(&inventory, Fixture::MonitorList, false, 40);

    // Assert
    assert_eq!(
        output,
        include_str!("snapshots/monitor-list-narrow.txt").trim_end()
    );
}

#[test]
fn test_eighty_column_table_wraps_unicode_names_without_losing_ids() {
    // Arrange
    let mut monitor = a_monitor();
    monitor.as_object_mut().unwrap().insert(
        "name".into(),
        json!("東京 very long service name with several descriptive words"),
    );

    // Act
    let output = plain(
        &json!({"items":[monitor], "next_cursor":null}),
        Fixture::MonitorList,
        false,
        80,
    );

    // Assert
    assert!(output.contains("NAME") && output.contains("東京"));
    assert!(output.contains("361aca90-733d-4b11-a398-ad961bfd7e48"));
    assert!(output.lines().all(|line| line.width() <= 80), "{output}");
}

#[test]
fn test_active_has_a_green_dot_without_claiming_health() {
    // Arrange
    let monitor = a_monitor();

    // Act
    let output = render(&monitor, Fixture::MonitorGet, false, 120, true);

    // Assert
    assert!(output.contains("\u{1b}[32m●") && output.contains("Active · HTTP"));
    assert!(!output.contains("Healthy") && !output.contains("\u{1b}[1mRbasovo"));
}

#[test]
fn test_terminal_and_bidi_controls_are_escaped_before_styling() {
    // Arrange
    let mut monitor = a_monitor();
    monitor
        .as_object_mut()
        .unwrap()
        .insert("name".into(), json!("bad\u{1b}[2J\nspoof\u{202e}"));

    // Act
    let output = plain(&monitor, Fixture::MonitorGet, false, 120);

    // Assert
    assert!(output.contains(r"bad\u{1b}[2J\nspoof\u{202e}"));
    assert!(!output.contains('\u{1b}') && !output.contains('\u{202e}'));
}

#[test]
fn test_creation_secret_is_shown_exactly_once_with_details() {
    // Arrange
    let value = json!({"monitor":a_monitor(), "heartbeat_secret":"one_time_fixture"});

    // Act
    let output = plain(&value, Fixture::Created, true, 120);

    // Assert
    assert_eq!(output.matches("one_time_fixture").count(), 1);
    assert!(output.contains("shown once; store securely"));
}

#[test]
fn test_mutation_summary_does_not_dump_audit_fields() {
    // Arrange
    let monitor = a_monitor();

    // Act
    let output = plain(&monitor, Fixture::Paused, false, 120);

    // Assert
    assert!(output.trim_start().starts_with("Paused monitor “Rbasovo”."));
    assert!(!output.contains("2026-09-07"));
}

#[test]
fn test_details_hide_sensitive_configuration_content() {
    // Arrange
    let mut monitor = a_monitor();
    let fields = monitor.as_object_mut().unwrap();
    fields.insert(
        "headers".into(),
        json!([{"name":"Authorization","value":"secret-header"}]),
    );
    fields.insert("body".into(), json!({"content":"secret-body"}));
    fields.insert("js_source".into(), json!("secret-script"));

    // Act
    let output = plain(&monitor, Fixture::MonitorGet, true, 120);

    // Assert
    assert!(!output.contains("secret-"));
    assert!(output.contains("values hidden") && output.contains("content hidden"));
}

#[test]
fn test_generic_error_details_redact_secret_objects_and_header_values() {
    // Arrange
    let error = CliError::Api {
        status: 400,
        code: "invalid".into(),
        message: "Invalid".into(),
        details: Some(
            json!({"token":{"value":"secret-token"}, "headers":[{"value":"secret-header"}]}),
        ),
        retry_after_secs: None,
    };

    // Act
    let output = super::error(&error, 120, false);

    // Assert
    assert!(!output.contains("secret-"));
    assert!(output.contains("[redacted]"));
}

#[test]
fn test_cron_heartbeat_shows_the_actual_api_schedule_field() {
    // Arrange
    let value = json!({"id":"361aca90-733d-4b11-a398-ad961bfd7e48","workspace_id":"00000000-0000-0000-0000-000000000001","name":"Backup", "config_type":"heartbeat", "state":"active",
        "schedule_type":"cron", "cron_expression":"0 9 * * *", "grace_secs":90});

    // Act
    let output = plain(&value, Fixture::MonitorGet, false, 120);

    // Assert
    assert!(output.contains("0 9 * * *") && output.contains("1 minute 30 seconds"));
    assert!(!output.contains("Timeout"));
}

#[test]
fn test_missing_required_fields_are_rejected_before_rendering() {
    // Arrange
    let value = json!({"name":"Partial"});
    // Act
    let result = Response::<Monitor>::parse(value);
    // Assert
    assert!(matches!(result, Err(CliError::MalformedResponse(_))));
}

#[test]
fn test_empty_inventory_retains_cursor_and_creation_hint() {
    // Arrange
    let value = json!({"items":[], "next_cursor":"cursor"});

    // Act
    let output = plain(&value, Fixture::MonitorList, false, 120);

    // Assert
    assert!(output.contains("No monitors found."));
    assert!(output.contains("monitor create http") && output.contains("--after cursor"));
}

#[test]
fn test_function_dry_run_preserves_verdict_and_failure_evidence() {
    // Arrange
    let value = json!({"type":"function", "status":"failed",
        "http":{"status":"passed","summary":"HTTP check passed","response":{"kind":"http","status":200,"latency_ms":42}},
        "function":{"status":"failed","summary":"Assertion failed","reason":"validation",
        "logs":[{"level":"error","message":"Missing field"}]}});

    // Act
    let output = plain(&value, Fixture::DryRun, false, 120);

    // Assert
    assert_eq!(output, include_str!("snapshots/dry-run.txt").trim_end());
}

#[test]
fn test_unavailable_execution_is_warning_not_success() {
    // Arrange
    let value = json!({"type":"http", "status":"execution_unavailable"});

    // Act
    let output = render(&value, Fixture::DryRun, false, 120, true);

    // Assert
    assert!(output.contains("Execution unavailable") && output.contains("[33m"));
    assert!(!output.contains("[32m"));
}

#[test]
fn test_entitlements_use_display_name_and_grouped_api_limits() {
    // Arrange
    let value = json!({"plan":{"display_name":"Developer","code":"developer","revision":{
        "entitlements":{"monitoring":{"minimum_check_interval_seconds":90},"api":{"requests_per_minute":100}}}}});

    // Act
    let output = plain(&value, Fixture::Entitlement, false, 120);

    // Assert
    assert!(output.contains("Developer") && output.contains("Monitoring"));
    assert!(output.contains("1 minute 30 seconds") && output.contains("100"));
}

#[test]
fn test_workspace_disconnect_explains_local_only_effect() {
    // Arrange
    let value = json!({"disconnected":"personal", "active":null, "api_key_revoked":false});

    // Act
    let output = plain(&value, Fixture::Disconnected, false, 120);

    // Assert
    assert!(output.contains("Disconnected workspace “personal”."));
    assert!(output.contains("API key remains valid on the server"));
}

#[test]
fn test_file_storage_is_neutral_without_a_repeated_warning() {
    // Arrange
    let value = json!({"alias":"personal","name":"Personal","active":true,"workspace_id":"00000000-0000-0000-0000-000000000001","slug":"personal",
        "api_url":"https://primary.fomkee.dev", "credential_store":"file",
        "credential_storage_notice":"Token stored unencrypted in the protected local credentials.toml file."});

    // Act
    let output = plain(&value, Fixture::Connected, false, 120);

    // Assert
    assert!(output.contains("Credentials") && output.contains("File"));
    assert!(!output.contains("Token stored unencrypted"));
}

#[test]
fn test_rate_limit_error_preserves_retry_after() {
    // Arrange
    let error = CliError::Api {
        status: 429,
        code: "rate_limited".into(),
        message: "Slow down".into(),
        details: None,
        retry_after_secs: Some(30),
    };

    // Act
    let output = super::error(&error, 120, false);

    // Assert
    assert!(output.contains("429 rate_limited") && output.contains("Retry after 30 seconds"));
}

fn unreadable_response(outcome: ResponseOutcome) -> CliError {
    use crate::error::response::{ResponseCause, ResponseFailure};
    CliError::Response(ResponseFailure {
        status: 201,
        outcome,
        retry_after_secs: None,
        source: ResponseCause::Decode(serde_json::from_str::<Value>("{").unwrap_err()),
    })
}

#[test]
fn test_acknowledged_mutation_error_explains_how_to_recover() {
    // Arrange
    let error = unreadable_response(ResponseOutcome::MutationAcknowledged);
    // Act
    let output = super::error(&error, 120, false);
    // Assert
    assert!(
        output.contains("do not retry automatically") && output.contains("fomkeecli monitor list")
    );
}

#[test]
fn test_failed_read_does_not_show_a_mutation_recovery_hint() {
    // Arrange
    let error = unreadable_response(ResponseOutcome::Read);
    // Act
    let output = super::error(&error, 120, false);
    // Assert
    assert!(!output.contains("mutation") && !output.contains("monitor list"));
}
