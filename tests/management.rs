#![cfg(test)]
mod management_support;
use axum::http::StatusCode;
use management_support::*;
use serde_json::{Value, json};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_monitor_update_fetches_detail_and_sends_preserving_replacement() {
    // Arrange
    let path = format!("monitors/{MONITOR}");
    let steps = vec![
        Step::new("GET", &path, Value::Null, monitor()),
        Step::new("PUT", &path, replacement(), monitor()),
    ];
    // Act
    let result = succeeded(
        run(
            steps,
            &["--json", "monitor", "update", MONITOR, "--name", "After"],
            "",
        )
        .await,
    );
    // Assert
    assert_eq!(result, monitor());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_complete_monitor_update_is_forwarded_without_detail_fetch() {
    let steps = vec![Step::new(
        "PUT",
        &format!("monitors/{MONITOR}"),
        replacement(),
        monitor(),
    )];
    let result = succeeded(
        run(
            steps,
            &["--json", "monitor", "update", MONITOR, "--file", "-"],
            &replacement().to_string(),
        )
        .await,
    );
    assert_eq!(result, monitor());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_creates_destination_from_stdin_without_exposing_input_secrets() {
    let input = json!({"name": "Chat", "channel_type": "telegram", "telegram_chat_id": "-100123", "telegram_bot_token": "secret-token"});
    let steps = vec![
        Step::new("POST", "alert-targets", input.clone(), destination())
            .status(StatusCode::CREATED),
    ];
    let result = succeeded(
        run(
            steps,
            &["--json", "destination", "create", "--file", "-"],
            &input.to_string(),
        )
        .await,
    );
    assert_eq!(result, destination());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_renames_destination_with_explicit_secret_preservation() {
    let path = format!("alert-targets/{TARGET}");
    let input = json!({"name": "Renamed", "channel_type": "telegram", "telegram_chat_id": "-100123", "telegram_bot_token": {"action": "preserve"}});
    let steps = vec![
        Step::new("GET", &path, Value::Null, destination()),
        Step::new("PUT", &path, input, destination()),
    ];
    let result = succeeded(
        run(
            steps,
            &[
                "--json",
                "destination",
                "update",
                TARGET,
                "--name",
                "Renamed",
            ],
            "",
        )
        .await,
    );
    assert_eq!(result, destination());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_lists_exactly_one_destination_page_with_encoded_cursor() {
    let page = json!({"items": [destination()], "next_cursor": "next", "future": true});
    let steps = vec![Step::new(
        "GET",
        "alert-targets?limit=2&after=a%2Fb",
        Value::Null,
        page.clone(),
    )];
    let result = succeeded(
        run(
            steps,
            &[
                "--json",
                "destination",
                "list",
                "--limit",
                "2",
                "--after",
                "a/b",
            ],
            "",
        )
        .await,
    );
    assert_eq!(result, page);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_lists_monitor_assignments_for_a_destination() {
    let page = json!({"items": [assignment()], "next_cursor": null});
    let steps = vec![Step::new(
        "GET",
        &format!("alert-targets/{TARGET}/alert-assignments?limit=100"),
        Value::Null,
        page.clone(),
    )];
    let result = succeeded(run(steps, &["--json", "destination", "monitors", TARGET], "").await);
    assert_eq!(result, page);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_lists_destinations_assigned_to_a_monitor() {
    let page = json!({"items": [assignment()], "next_cursor": null});
    let steps = vec![Step::new(
        "GET",
        &format!("monitors/{MONITOR}/alert-assignments?limit=100"),
        Value::Null,
        page.clone(),
    )];
    let result = succeeded(run(steps, &["--json", "monitor", "destinations", MONITOR], "").await);
    assert_eq!(result, page);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_assigns_destination_to_monitor() {
    let steps = vec![
        Step::new(
            "POST",
            &format!("monitors/{MONITOR}/alert-assignments"),
            json!({"alert_target_id": TARGET}),
            assignment(),
        )
        .status(StatusCode::CREATED),
    ];
    let result = succeeded(
        run(
            steps,
            &["--json", "destination", "assign", TARGET, MONITOR],
            "",
        )
        .await,
    );
    assert_eq!(result, assignment());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_removes_assignment_by_its_own_identifier() {
    let steps = vec![
        Step::new(
            "DELETE",
            &format!("alert-assignments/{BINDING}"),
            Value::Null,
            Value::Null,
        )
        .status(StatusCode::NO_CONTENT),
    ];
    let result = succeeded(run(steps, &["--json", "destination", "unassign", BINDING], "").await);
    assert_eq!(result, json!({"removed": true, "assignment_id": BINDING}));
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_destination_test_returns_actual_provider_outcome() {
    let outcome = json!({"id": BINDING, "alert_target_id": TARGET, "channel_kind": "telegram", "outcome": "failed", "attempted_at": "2026-09-15T00:00:00Z"});
    let steps = vec![Step::new(
        "POST",
        &format!("alert-targets/{TARGET}/test"),
        Value::Null,
        outcome.clone(),
    )];
    let result = succeeded(run(steps, &["--json", "destination", "test", TARGET], "").await);
    assert_eq!(result, outcome);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_does_not_retry_failed_destination_mutations() {
    let steps = vec![
        Step::new(
            "POST",
            &format!("alert-targets/{TARGET}/test"),
            Value::Null,
            json!({"code":"unavailable","message":"try later"}),
        )
        .status(StatusCode::SERVICE_UNAVAILABLE),
    ];
    let output = run(steps, &["--json", "destination", "test", TARGET], "").await;
    assert_eq!(output.status.code(), Some(5));
    assert!(output.stdout.is_empty());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_unreadable_destination_mutation_retains_acknowledged_outcome() {
    let steps = vec![Step::new(
        "POST",
        &format!("alert-targets/{TARGET}/test"),
        Value::Null,
        json!({"invalid": true}),
    )];
    let output = run(steps, &["--json", "destination", "test", TARGET], "").await;
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        (output.status.code(), error.get("code").unwrap().as_str()),
        (Some(8), Some("mutation_result_unavailable"))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_complete_destination_update_is_forwarded_without_detail_fetch() {
    // Arrange
    let input = json!({"name":"Chat","channel_type":"telegram","telegram_chat_id":"123","telegram_bot_token":{"action":"replace","value":"new-private-token"}});
    let steps = vec![Step::new(
        "PUT",
        &format!("alert-targets/{TARGET}"),
        input.clone(),
        destination(),
    )];
    // Act
    let result = succeeded(
        run(
            steps,
            &["--json", "destination", "update", TARGET, "--file", "-"],
            &input.to_string(),
        )
        .await,
    );
    // Assert
    assert_eq!(result, destination());
}
