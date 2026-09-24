use super::http_support::*;
use serde_json::json;
use std::fs;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_url_only_http_create_uses_api_interval_and_omits_server_defaults() {
    // Arrange
    let server = ApiServer::with_minimum(json!(150)).await;
    let process = Process::new();

    // Act
    let output = process
        .authorized(&server)
        .args(["monitor", "create", "http", "https://example.com/health"])
        .output()
        .unwrap();

    // Assert
    assert!(output.status.success(), "{output:?}");
    server.assert_created_input(json!({"config_type":"http", "name":"example.com/health", "url":"https://example.com/health", "tags":[], "interval_secs":150})).await;
    server.finish(3).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_explicit_interval_skips_entitlement_discovery_and_leaves_validation_to_api() {
    // Arrange
    let server = ApiServer::start().await;
    let process = Process::new();

    // Act
    let output = process
        .authorized(&server)
        .args([
            "monitor",
            "create",
            "http",
            "https://example.com/",
            "--interval",
            "1s",
            "--name",
            "API",
            "--method",
            "HEAD",
        ])
        .output()
        .unwrap();

    // Assert
    assert!(output.status.success(), "{output:?}");
    server.assert_created_input(json!({"config_type":"http", "name":"API", "url":"https://example.com/", "tags":[], "interval_secs":1, "method":"HEAD"})).await;
    server.finish(2).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_invalid_entitlement_response_stops_before_creation() {
    // Arrange
    let server = ApiServer::with_minimum(json!(null)).await;
    let process = Process::new();

    // Act
    let output = process
        .authorized(&server)
        .args(["monitor", "create", "http", "https://example.com/"])
        .output()
        .unwrap();

    // Assert
    assert_failure(&output, 8, "protocol");
    server.finish(2).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_function_creation_reads_script_and_uses_api_interval() {
    // Arrange
    let server = ApiServer::start().await;
    let process = Process::new();
    let source = process.directory.path().join("check.js");
    fs::write(&source, "// fixture script").unwrap();

    // Act
    let output = process
        .authorized(&server)
        .args([
            "monitor",
            "create",
            "function",
            "https://example.com",
            "--script",
        ])
        .arg(source)
        .output()
        .unwrap();

    // Assert
    assert!(output.status.success(), "{output:?}");
    server.assert_created_input(json!({"config_type":"function", "name":"example.com", "url":"https://example.com", "tags":[], "interval_secs":60, "js_source":"// fixture script"})).await;
    server.finish(3).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_heartbeat_creation_translates_schedule_and_leaves_grace_to_api() {
    // Arrange
    let server = ApiServer::start().await;
    let process = Process::new();

    // Act
    let output = process
        .authorized(&server)
        .args([
            "monitor",
            "create",
            "heartbeat",
            "--name",
            "backup",
            "--every",
            "24h",
        ])
        .output()
        .unwrap();

    // Assert
    assert!(output.status.success(), "{output:?}");
    server.assert_created_input(json!({"config_type":"heartbeat", "name":"backup", "tags":[], "schedule_type":"interval", "period_secs":86400})).await;
    server.finish(2).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_file_input_is_forwarded_without_injecting_cli_defaults() {
    // Arrange
    let server = ApiServer::start().await;
    let process = Process::new();
    let path = process.directory.path().join("monitor.json");
    let request = json!({"config_type":"http", "name":"From file", "url":"https://example.com", "interval_secs":300});
    fs::write(&path, request.to_string()).unwrap();

    // Act
    let output = process
        .authorized(&server)
        .args(["monitor", "create", "http", "--file"])
        .arg(path)
        .output()
        .unwrap();

    // Assert
    assert!(output.status.success(), "{output:?}");
    server.assert_created_input(request).await;
    server.finish(2).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_default_creation_output_is_human_readable_even_when_captured() {
    // Arrange
    let server = ApiServer::start().await;
    let process = Process::new();

    // Act
    let output = process
        .command()
        .env("FOMKEE_API_TOKEN", TOKEN)
        .env("FOMKEE_API_URL", &server.origin)
        .args(["monitor", "create", "http", "https://example.com/health"])
        .output()
        .unwrap();

    // Assert
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success());
    assert!(stdout.contains("Created monitor “example.com/health”"));
    assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_err());
    server.finish(3).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_detailed_creation_does_not_make_extra_api_requests() {
    // Arrange
    let server = ApiServer::start().await;
    let process = Process::new();

    // Act
    let output = process
        .command()
        .env("FOMKEE_API_TOKEN", TOKEN)
        .env("FOMKEE_API_URL", &server.origin)
        .args([
            "monitor",
            "create",
            "http",
            "https://example.com/health",
            "--details",
            "--color=never",
        ])
        .output()
        .unwrap();

    // Assert
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("References") && stdout.contains("Every"));
    server.finish(3).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_saved_workspace_name_is_rendered_without_a_decoration_lookup() {
    // Arrange
    let server = ApiServer::start().await;
    let process = Process::new();
    assert!(process.connect_stdin(&server, TOKEN).status.success());

    // Act
    let output = process
        .command()
        .args([
            "monitor",
            "create",
            "http",
            "https://example.com/health",
            "--interval",
            "1m",
            "--details",
        ])
        .output()
        .unwrap();

    // Assert
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!("Workspace  Personal ({WORKSPACE})")));
    assert!(!stdout.contains("app.fomkee.com") && !stdout.contains("stored unencrypted"));
    server.finish(4).await;
}
