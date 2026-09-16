use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_lists_one_incident_page_and_preserves_extensions() {
    // Arrange
    let expected = page(incident());
    let steps = vec![Step::new(
        "GET",
        "incidents?limit=2&after=a%2Fb",
        Value::Null,
        expected.clone(),
    )];
    // Act
    let output = run(
        steps,
        &[
            "incident", "list", "--limit", "2", "--after", "a/b", "--json",
        ],
        "",
    )
    .await;
    // Assert
    assert_eq!(succeeded(output), expected);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_lists_incidents_for_selected_monitor() {
    // Arrange
    let expected = page(incident());
    let steps = vec![Step::new(
        "GET",
        &format!("monitors/{MONITOR}/incidents?limit=100"),
        Value::Null,
        expected.clone(),
    )];
    // Act
    let output = run(
        steps,
        &["incident", "list", "--monitor", MONITOR, "--json"],
        "",
    )
    .await;
    // Assert
    assert_eq!(succeeded(output), expected);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_gets_incident_without_extra_reads() {
    // Arrange
    let steps = vec![Step::new(
        "GET",
        &format!("incidents/{INCIDENT}"),
        Value::Null,
        incident(),
    )];
    // Act
    let output = run(
        steps,
        &[
            "incident",
            "get",
            INCIDENT,
            "--json",
            "--details",
            "--color",
            "always",
        ],
        "",
    )
    .await;
    // Assert
    assert_eq!(succeeded(output), incident());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_reads_timeline_including_public_notes() {
    // Arrange
    let expected = page(note());
    let steps = vec![Step::new(
        "GET",
        &format!("incidents/{INCIDENT}/timeline?limit=1&after=a%2Fb"),
        Value::Null,
        expected.clone(),
    )];
    // Act
    let output = run(
        steps,
        &[
            "incident", "timeline", INCIDENT, "--limit", "1", "--after", "a/b", "--json",
        ],
        "",
    )
    .await;
    // Assert
    assert_eq!(succeeded(output), expected);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_publishes_one_note_from_stdin_without_changing_incident_status() {
    // Arrange
    let steps = vec![
        Step::new(
            "POST",
            &format!("incidents/{INCIDENT}/updates"),
            json!({"message": "Investigating the outage."}),
            note(),
        )
        .status(StatusCode::CREATED),
    ];
    // Act
    let output = run(
        steps,
        &[
            "incident",
            "post",
            INCIDENT,
            "--message-file",
            "-",
            "--json",
        ],
        "Investigating the outage.",
    )
    .await;
    // Assert
    assert_eq!(succeeded(output), note());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_publishes_note_from_message_flag() {
    // Arrange
    let steps = vec![Step::new(
        "POST",
        &format!("incidents/{INCIDENT}/updates"),
        json!({"message": "Investigating the outage."}),
        note(),
    )];
    // Act
    let output = run(
        steps,
        &[
            "incident",
            "post",
            INCIDENT,
            "--message",
            "Investigating the outage.",
            "--json",
        ],
        "",
    )
    .await;
    // Assert
    assert_eq!(succeeded(output), note());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_does_not_retry_rejected_publication() {
    // Arrange
    let steps = vec![
        Step::new(
            "POST",
            &format!("incidents/{INCIDENT}/updates"),
            json!({"message":"Update"}),
            json!({"error":"forbidden", "code":"insufficient_permissions"}),
        )
        .status(StatusCode::FORBIDDEN),
    ];
    // Act
    let output = run(
        steps,
        &[
            "incident",
            "post",
            INCIDENT,
            "--message",
            "Update",
            "--json",
        ],
        "",
    )
    .await;
    // Assert
    assert!(!output.status.success() && output.stdout.is_empty());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_reports_ambiguous_publication_result_without_reposting() {
    // Arrange
    let steps = vec![
        Step::new(
            "POST",
            &format!("incidents/{INCIDENT}/updates"),
            json!({"message":"Update"}),
            json!({"type":"opened"}),
        )
        .status(StatusCode::CREATED),
    ];
    // Act
    let output = run(
        steps,
        &[
            "incident",
            "post",
            INCIDENT,
            "--message",
            "Update",
            "--json",
        ],
        "",
    )
    .await;
    // Assert
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("mutation_result_unavailable")
    );
}
