#![cfg(test)]

use fomkee_cli::client::{FomkeeApi, InMemoryFomkeeApi};
use fomkee_cli::dto::{AuthStatus, Entitlements, Monitor, MonitorConfig};
use fomkee_cli::error::CliError;
use fomkee_cli::monitoring::{CreateInput, DryRunInput, LifecycleInput};
use fomkee_cli::wire::Response;
use serde_json::{Value, json};

const WORKSPACE: &str = "00000000-0000-0000-0000-000000000001";
const MONITOR: &str = "00000000-0000-0000-0000-000000000002";

fn request() -> Value {
    json!({"config_type":"http", "name":"Example", "url":"https://example.com",
        "interval_secs":60, "auth":{"type":"basic","username":"secret-user","password":"secret-password"},
        "headers":[{"name":"Authorization","value":"secret-header"}],
        "body":{"format":"plain","content":"secret-body"}, "extension":{"nested":"secret-extension"}})
}

fn monitor() -> Value {
    json!({"id":MONITOR,"workspace_id":WORKSPACE,"name":"Example","state":"active",
        "config_type":"http","method":"GET","url":"https://example.com","interval_secs":60,
        "auth":{"type":"basic"},"extension":{"null":null,"items":[1,2]}, "description":null})
}

#[test]
fn test_create_debug_redacts_nested_credentials_body_headers_and_extensions() {
    // Arrange
    let input = CreateInput::try_from(request()).unwrap();
    // Act
    let debug = format!("{input:?}");
    // Assert
    assert_eq!(debug, "CreateInput([REDACTED])");
}

#[test]
fn test_typed_creation_preserves_complete_request_and_extension_fields() {
    // Arrange
    let original = request();
    // Act
    let encoded = serde_json::to_value(CreateInput::try_from(original.clone()).unwrap()).unwrap();
    // Assert
    assert_eq!(encoded, original);
}

#[test]
fn test_complete_file_request_cannot_be_overridden_by_convenience_defaults() {
    // Arrange
    let mut input = CreateInput::try_from(request()).unwrap();
    let limits: Entitlements = serde_json::from_value(json!({"plan":{"revision":{"entitlements":{
        "monitoring":{"minimum_check_interval_seconds":90}}}}}))
    .unwrap();
    // Act
    let result = input.use_api_interval(&limits);
    // Assert
    assert!(matches!(result, Err(CliError::InvalidInput(_))));
}

#[test]
fn test_response_preserves_unknown_and_null_fields_in_json() {
    // Arrange
    let original = monitor();
    // Act
    let response = Response::<Monitor>::parse(original.clone()).unwrap();
    // Assert
    assert_eq!(serde_json::to_value(response).unwrap(), original);
}

#[test]
fn test_authentication_status_does_not_require_write_only_credentials() {
    // Arrange
    let value = monitor();
    // Act
    let response = Response::<Monitor>::parse(value).unwrap();
    // Assert
    assert!(
        matches!(&response.data().config, MonitorConfig::Http { active, .. }
        if matches!(active.auth, Some(AuthStatus::Basic)))
    );
}

#[test]
fn test_api_key_authentication_request_is_supported_and_redacted() {
    // Arrange
    let value = json!({"config_type":"http","auth":{"type":"api_key","header_name":"X-Key","value":"secret-key"}});
    // Act
    let input = CreateInput::try_from(value.clone()).unwrap();
    // Assert
    assert_eq!(format!("{input:?}"), "CreateInput([REDACTED])");
    assert_eq!(serde_json::to_value(input).unwrap(), value);
}

#[test]
fn test_response_debug_does_not_expose_unknown_secret_fields() {
    // Arrange
    let value = json!({"workspace_id":WORKSPACE,"extension":"secret-unknown"});
    // Act
    let response = Response::<fomkee_cli::dto::Session>::parse(value).unwrap();
    // Assert
    assert_eq!(format!("{response:?}"), "Response([REDACTED])");
}

#[test]
fn test_dry_run_input_rejects_heartbeat_before_any_api_call() {
    // Arrange
    let value = json!({"config_type":"heartbeat","schedule_type":"interval","period_secs":60});
    // Act
    let result = DryRunInput::try_from(value);
    // Assert
    assert!(matches!(result, Err(CliError::InvalidInput(_))));
}

#[test]
fn test_lifecycle_debug_redacts_free_form_reason() {
    // Arrange
    let input = LifecycleInput::pause(Some("secret-reason".into()), None);
    // Act
    let debug = format!("{input:?}");
    // Assert
    assert!(!debug.contains("secret-reason") && debug.contains("REDACTED"));
}

#[tokio::test]
async fn test_unconfigured_monitor_lookup_is_an_error_not_empty_success() {
    // Arrange
    let api = InMemoryFomkeeApi::with_session(WORKSPACE.parse().unwrap());
    // Act
    let result = api
        .get_monitor(&WORKSPACE.parse().unwrap(), &MONITOR.parse().unwrap())
        .await;
    // Assert
    assert!(matches!(result, Err(CliError::Configuration(_))));
}

#[tokio::test]
async fn test_unconfigured_creation_is_an_error_not_empty_success() {
    // Arrange
    let api = InMemoryFomkeeApi::with_session(WORKSPACE.parse().unwrap());
    // Act
    let result = api
        .create_monitor(
            &WORKSPACE.parse().unwrap(),
            CreateInput::try_from(request()).unwrap(),
        )
        .await;
    // Assert
    assert!(matches!(result, Err(CliError::Configuration(_))));
}

#[tokio::test]
async fn test_unconfigured_deletion_does_not_pretend_to_delete() {
    // Arrange
    let api = InMemoryFomkeeApi::with_session(WORKSPACE.parse().unwrap());
    // Act
    let result = api
        .delete_monitor(&WORKSPACE.parse().unwrap(), &MONITOR.parse().unwrap())
        .await;
    // Assert
    assert!(matches!(result, Err(CliError::Configuration(_))));
}

#[test]
fn test_auth_metadata_replaces_a_server_extension_without_duplicate_json_keys() {
    // Arrange
    use fomkee_cli::config::{ApiToken, Config, api_url};
    use fomkee_cli::workspace::{Connection, ConnectionOrigin};
    let connection = Connection {
        config: Config {
            base_url: api_url("https://primary.fomkee.dev").unwrap(),
            token: ApiToken::new("fk_fixture".into()).unwrap(),
        },
        origin: ConnectionOrigin::Environment,
    };
    let session = Response::parse(
        json!({"workspace_id":WORKSPACE,"connection":{"untrusted":true},"future":null}),
    )
    .unwrap();
    // Act
    let encoded = serde_json::to_string(&connection.status(session)).unwrap();
    // Assert
    assert_eq!(encoded.matches("\"connection\"").count(), 1);
    assert!(!encoded.contains("untrusted") && encoded.contains("\"future\":null"));
}
