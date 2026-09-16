use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_creates_maintenance_with_explicit_timezone_and_monitors() {
    // Arrange
    let steps =
        vec![Step::new("POST", "maintenance", settings(), window()).status(StatusCode::CREATED)];
    // Act
    let output = run(
        steps,
        &[
            "maintenance",
            "create",
            "--title",
            "Database upgrade",
            "--start",
            START,
            "--end",
            END,
            "--monitor",
            MONITOR,
            "--json",
        ],
        "",
    )
    .await;
    // Assert
    assert_eq!(succeeded(output), window());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_preserves_complete_maintenance_file_fields() {
    // Arrange
    let mut input = settings();
    input
        .as_object_mut()
        .unwrap()
        .insert("future".into(), json!(true));
    let steps = vec![Step::new("POST", "maintenance", input.clone(), window())];
    // Act
    let output = run(
        steps,
        &["maintenance", "create", "--file", "-", "--json"],
        &input.to_string(),
    )
    .await;
    // Assert
    assert_eq!(succeeded(output), window());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_lists_exactly_one_maintenance_page() {
    // Arrange
    let expected = page(window());
    let steps = vec![Step::new(
        "GET",
        "maintenance?limit=1&after=a%2Fb",
        Value::Null,
        expected.clone(),
    )];
    // Act
    let output = run(
        steps,
        &[
            "maintenance",
            "list",
            "--limit",
            "1",
            "--after",
            "a/b",
            "--json",
        ],
        "",
    )
    .await;
    // Assert
    assert_eq!(succeeded(output), expected);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cancels_maintenance_once() {
    // Arrange
    let mut cancelled = window();
    cancelled
        .as_object_mut()
        .unwrap()
        .insert("state".into(), json!("cancelled"));
    let steps = vec![Step::new(
        "POST",
        &format!("maintenance/{MAINTENANCE}/cancel"),
        Value::Null,
        cancelled.clone(),
    )];
    // Act
    let output = run(steps, &["maintenance", "cancel", MAINTENANCE, "--json"], "").await;
    // Assert
    assert_eq!(succeeded(output), cancelled);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rejects_file_timestamp_without_timezone_before_mutation() {
    // Arrange
    let mut input = settings();
    input
        .as_object_mut()
        .unwrap()
        .insert("scheduled_start".into(), json!("2030-10-01T09:00:00"));
    // Act
    let output = run(
        vec![],
        &["maintenance", "create", "--file", "-", "--json"],
        &input.to_string(),
    )
    .await;
    // Assert
    assert!(!output.status.success() && output.stdout.is_empty());
}
