#![cfg(test)]

mod workspace_support;

use std::fs;

use fomkee_cli::config::ApiToken;
use fomkee_cli::error::CliError;
use fomkee_cli::workspace::credentials::{CredentialStore, InMemoryCredentialStore};
use fomkee_cli::workspace::model::CredentialId;
use fomkee_cli::workspace::service;
use fomkee_cli::workspace::store::{FileWorkspaceStore, InMemoryWorkspaceStore, WorkspaceStore};
use serde_json::json;
use tempfile::tempdir;

use workspace_support::*;

#[tokio::test]
async fn test_connect_discovers_identity_and_selects_first_workspace() {
    // Arrange
    let scenario = Scenario::default();

    // Act
    let result = scenario.connect("personal", FIRST).await.unwrap();

    // Assert
    assert_eq!(result, expected_connection("personal", FIRST, true));
    assert_saved_connection(&scenario, "personal", FIRST);
}

#[tokio::test]
async fn test_second_connection_preserves_active_workspace() {
    // Arrange
    let scenario = two_workspaces().await;

    // Act
    let result = serde_json::to_value(service::list(&scenario.store).unwrap()).unwrap();

    // Assert
    assert_eq!(
        result,
        json!({"active": "personal", "workspaces": [
            expected_connection("company", SECOND, false), expected_connection("personal", FIRST, true)
        ]})
    );
}

#[tokio::test]
async fn test_switch_selects_saved_workspace_and_credential() {
    // Arrange
    let scenario = two_workspaces().await;

    // Act
    service::use_workspace(&scenario.store, &alias("company")).unwrap();

    // Assert
    let (selected, profile) = service::selected_profile(&scenario.store, None).unwrap();
    assert_eq!(
        (selected, profile.workspace_id.to_string()),
        (alias("company"), SECOND.into())
    );
    assert_saved_connection(&scenario, "company", SECOND);
}

#[tokio::test]
async fn test_explicit_workspace_overrides_active_without_changing_default() {
    // Arrange
    let scenario = two_workspaces().await;

    // Act
    let (selected, _) =
        service::selected_profile(&scenario.store, Some(&alias("company"))).unwrap();

    // Assert
    assert_eq!(selected, alias("company"));
    assert_eq!(
        scenario.store.load().unwrap().active,
        Some(alias("personal"))
    );
}

#[tokio::test]
async fn test_duplicate_alias_cannot_overwrite_existing_connection() {
    // Arrange
    let scenario = two_workspaces().await;

    // Act
    let result = scenario.connect("personal", SECOND).await;

    // Assert
    assert!(matches!(result, Err(CliError::InvalidInput(_))));
    assert_saved_connection(&scenario, "personal", FIRST);
}

#[tokio::test]
async fn test_unknown_alias_never_falls_back_to_active_workspace() {
    // Arrange
    let scenario = two_workspaces().await;

    // Act
    let result = service::selected_profile(&scenario.store, Some(&alias("unknown")));

    // Assert
    assert!(matches!(result, Err(CliError::InvalidInput(_))));
}

#[tokio::test]
async fn test_disconnect_removes_credential_and_does_not_choose_another_workspace() {
    // Arrange
    let scenario = two_workspaces().await;
    let (_, profile) = service::selected_profile(&scenario.store, None).unwrap();

    // Act
    service::disconnect(&scenario.store, &scenario.stores(), &alias("personal")).unwrap();

    // Assert
    assert_disconnected(&scenario, &profile.credential_id);
}

#[tokio::test]
async fn test_disconnect_can_clean_up_a_missing_saved_credential() {
    // Arrange
    let scenario = two_workspaces().await;
    let (_, profile) = service::selected_profile(&scenario.store, None).unwrap();
    scenario.credentials.delete(&profile.credential_id).unwrap();

    // Act
    let result = service::disconnect(&scenario.store, &scenario.stores(), &alias("personal"));

    // Assert
    assert!(result.is_ok());
    assert_disconnected(&scenario, &profile.credential_id);
}

#[tokio::test]
async fn test_invalid_session_does_not_save_a_connection() {
    // Arrange
    let scenario = Scenario::default();

    // Act
    let result = scenario.connect("personal", "not-a-workspace-id").await;

    // Assert
    assert!(matches!(result, Err(CliError::MalformedResponse(_))));
    assert!(scenario.store.load().unwrap().workspaces.is_empty());
}

#[tokio::test]
async fn test_credential_store_failure_does_not_save_a_connection() {
    // Arrange
    let store = InMemoryWorkspaceStore::default();

    // Act
    let result = connect_with(&store, &UnavailableCredentials).await;

    // Assert
    assert!(matches!(result, Err(CliError::CredentialStore)));
    assert!(store.load().unwrap().workspaces.is_empty());
}

#[tokio::test]
async fn test_profile_save_failure_removes_newly_saved_credential() {
    // Arrange
    let credentials = RecordingCredentials::default();

    // Act
    let result = connect_with(&UnwritableStore, &credentials).await;

    // Assert
    assert!(matches!(result, Err(CliError::Configuration(_))));
    assert_credential_rolled_back(&credentials);
}

#[test]
fn test_file_store_satisfies_registry_contract() {
    // Arrange
    let directory = tempdir().unwrap();
    let store = FileWorkspaceStore::open(directory.path().to_owned()).unwrap();

    // Act / Assert
    assert_store_contract(&store);
}

#[test]
fn test_memory_store_satisfies_registry_contract() {
    // Arrange
    let store = InMemoryWorkspaceStore::default();

    // Act / Assert
    assert_store_contract(&store);
}

#[test]
fn test_registry_survives_reopening_and_contains_no_token() {
    // Arrange
    let directory = tempdir().unwrap();
    save_fixture_registry(directory.path());

    // Act
    let store = FileWorkspaceStore::open(directory.path().to_owned()).unwrap();

    // Assert
    assert_eq!(store.load().unwrap().active, Some(alias("personal")));
    assert_metadata_has_no_secret(directory.path());
}

#[test]
fn test_simultaneous_profile_edits_fail_without_losing_configuration() {
    // Arrange
    let directory = tempdir().unwrap();
    let store = FileWorkspaceStore::open(directory.path().to_owned()).unwrap();

    // Act
    let result = FileWorkspaceStore::open(directory.path().to_owned());

    // Assert
    assert!(matches!(result, Err(CliError::Configuration(_))));
    assert_store_contract(&store);
}

#[test]
fn test_corrupt_configuration_is_not_silently_replaced() {
    // Arrange
    let directory = tempdir().unwrap();
    fs::write(directory.path().join("workspaces.toml"), "[broken").unwrap();
    let store = FileWorkspaceStore::open(directory.path().to_owned()).unwrap();

    // Act
    let result = store.load();

    // Assert
    assert!(matches!(result, Err(CliError::Configuration(_))));
}

#[test]
fn test_session_mismatch_blocks_saved_workspace() {
    // Arrange
    let connection = saved_connection(FIRST);

    // Act
    let result = connection.verify_session(
        &fomkee_cli::wire::Response::parse(json!({"workspace_id": SECOND})).unwrap(),
    );

    // Assert
    assert!(matches!(result, Err(CliError::InvalidInput(_))));
}

#[test]
fn test_tokens_are_trimmed_and_redacted() {
    // Arrange
    let token = ApiToken::new(format!("  {TOKEN}\r\n")).unwrap();
    let credentials = InMemoryCredentialStore::default();
    let id = CredentialId::generate().unwrap();

    // Act
    credentials.set(&id, &token).unwrap();

    // Assert
    assert_eq!(
        format!("{:?}", credentials.get(&id).unwrap()),
        "ApiToken([REDACTED])"
    );
}

#[test]
fn test_legacy_json_migrates_to_toml_without_changing_keyring_credentials() {
    // Arrange
    let directory = tempdir().unwrap();
    save_legacy_registry(directory.path());
    let store = FileWorkspaceStore::open(directory.path().to_owned()).unwrap();

    // Act
    let registry = store.load().unwrap();

    // Assert
    assert_eq!(registry.active, Some(alias("personal")));
    assert_metadata_has_no_secret(directory.path());
    assert!(directory.path().join("workspaces.json").exists());
}

#[test]
fn test_corrupt_toml_does_not_restore_stale_legacy_json() {
    // Arrange
    let directory = tempdir().unwrap();
    save_legacy_registry(directory.path());
    fs::write(directory.path().join("workspaces.toml"), "[broken").unwrap();
    let store = FileWorkspaceStore::open(directory.path().to_owned()).unwrap();

    // Act
    let result = store.load();

    // Assert
    assert!(matches!(result, Err(CliError::Configuration(_))));
}
