use super::*;

#[test]
fn test_builds_http_creation_from_convenience_flags() {
    // Arrange
    let command = CreateCommand::Http(HttpCreateArgs {
        common: common_args(),
        url: Some("https://example.com/health".into()),
        interval_secs: Some(60),
        method: Some("GET".into()),
    });

    // Act
    let result = command.into_input();

    // Assert
    assert!(result.is_ok(), "creation failed: {result:?}");
    assert_eq!(
        result.ok().and_then(|input| serde_json::to_value(input)
            .unwrap()
            .get("config_type")
            .cloned()),
        Some(serde_json::json!("http"))
    );
}

#[test]
fn test_builds_function_creation_from_source_file() -> Result<(), Box<dyn std::error::Error>> {
    // Arrange
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("check.js");
    std::fs::write(
        &source,
        "if (response.status !== 200) throw new Error('failed');",
    )?;
    let command = CreateCommand::Function(FunctionCreateArgs {
        common: common_args(),
        url: Some("https://example.com".into()),
        interval_secs: Some(60),
        js_source_file: source.to_str().map(str::to_owned),
    });

    // Act
    let result = command.into_input()?;

    // Assert
    assert!(
        serde_json::to_value(result)
            .unwrap()
            .get("js_source")
            .and_then(Value::as_str)
            .is_some_and(|value| value.contains("response.status"))
    );
    Ok(())
}

#[test]
fn test_builds_heartbeat_creation_with_interval_schedule() {
    // Arrange
    let command = CreateCommand::Heartbeat(HeartbeatCreateArgs {
        common: common_args(),
        period_secs: Some(300),
        cron_expression: None,
        grace_secs: Some(30),
        js_source_file: None,
    });

    // Act
    let result = command.into_input();

    // Assert
    assert!(result.is_ok(), "creation failed: {result:?}");
    assert_eq!(
        result.ok().and_then(|input| serde_json::to_value(input)
            .unwrap()
            .get("schedule_type")
            .cloned()),
        Some(serde_json::json!("interval"))
    );
}

fn common_args() -> CommonCreateArgs {
    CommonCreateArgs {
        name: Some("Example".into()),
        description: None,
        tags: vec!["production".into()],
        file: None,
    }
}
