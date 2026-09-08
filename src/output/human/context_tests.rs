use super::fixtures::{Fixture, render};
use super::*;
use crate::dto::Monitor;
use crate::wire::Response;
use serde_json::{Value, json};

const WORKSPACE: &str = "00000000-0000-0000-0000-000000000001";
const MONITOR: &str = "361aca90-733d-4b11-a398-ad961bfd7e48";

fn monitor() -> Value {
    json!({"id":MONITOR, "workspace_id":WORKSPACE, "name":"Example",
        "state":"active", "config_type":"http", "method":"GET", "url":"https://example.com", "interval_secs":60, "created_by":"unhelpful-creator-id",
        "auth":{"type":"none"}, "headers":[], "body":null})
}

fn saved_context(hosted: bool) -> DisplayContext {
    DisplayContext {
        workspace: Some((WORKSPACE.parse().unwrap(), "Personal".into())),
        hosted,
    }
}

#[test]
fn test_named_workspace_and_hosted_link_are_shown_in_references() {
    // Arrange
    let context = saved_context(true);

    // Act
    let output = render(&monitor(), Fixture::MonitorGet, true, 120, false, &context);

    // Assert
    assert!(output.contains(&format!("Workspace  Personal ({WORKSPACE})")));
    assert!(output.contains(&format!(
        "In Fomkee\n    https://app.fomkee.dev/monitors/{MONITOR}"
    )));
}

#[test]
fn test_workspace_id_is_secondary_when_color_is_enabled() {
    // Arrange
    let context = saved_context(true);

    // Act
    let output = render(&monitor(), Fixture::MonitorGet, true, 120, true, &context);

    // Assert
    assert!(output.contains(&format!("Personal\u{1b}[2m ({WORKSPACE})")));
}

#[test]
fn test_empty_request_and_unnamed_creator_are_omitted_with_details() {
    // Arrange
    let context = saved_context(true);

    // Act
    let output = render(&monitor(), Fixture::MonitorGet, true, 120, false, &context);

    // Assert
    assert!(!output.contains("Request") && !output.contains("unhelpful-creator-id"));
    assert!(!output.contains("Created by"));
}

#[test]
fn test_custom_api_origin_does_not_create_a_hosted_app_link() {
    // Arrange
    let context = saved_context(false);

    // Act
    let output = render(&monitor(), Fixture::MonitorGet, false, 120, false, &context);

    // Assert
    assert!(!output.contains("app.fomkee.dev") && !output.contains("In Fomkee"));
}

#[test]
fn test_environment_connection_does_not_display_a_bare_workspace_id() {
    // Arrange
    let context = DisplayContext {
        workspace: None,
        hosted: true,
    };

    // Act
    let output = render(&monitor(), Fixture::MonitorGet, true, 120, false, &context);

    // Assert
    assert!(!output.contains(WORKSPACE));
    assert!(output.contains("app.fomkee.dev"));
}

#[test]
fn test_name_from_another_workspace_is_not_shown() {
    // Arrange
    let context = DisplayContext {
        workspace: Some((
            "00000000-0000-0000-0000-000000000002".parse().unwrap(),
            "Wrong".into(),
        )),
        hosted: true,
    };

    // Act
    let output = render(&monitor(), Fixture::MonitorGet, true, 120, false, &context);

    // Assert
    assert!(!output.contains("Wrong") && !output.contains("Workspace"));
}

#[test]
fn test_invalid_monitor_id_cannot_become_a_link() {
    // Arrange
    let value = json!({"id":"../../fake?token=bad", "name":"Bad"});

    // Act
    let result = Response::<Monitor>::parse(value);
    // Assert
    assert!(matches!(result, Err(CliError::MalformedResponse(_))));
}

#[test]
fn test_page_starts_with_status_and_finishes_with_breathing_room() {
    // Arrange
    let context = saved_context(true);

    // Act
    let output = render(&monitor(), Fixture::MonitorGet, false, 120, false, &context);

    // Assert
    assert!(output.starts_with("\n  ● Active · HTTP\n"));
    assert!(output.ends_with('\n') && output.contains("\n  References\n    Name"));
}
