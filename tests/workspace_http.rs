#![cfg(test)]

#[path = "http_creation/mod.rs"]
mod creation;
mod http_support;

use fomkee_cli::client::HttpFomkeeApi;
use fomkee_cli::workspace::credential_stores::CredentialStores;
use fomkee_cli::workspace::credentials::InMemoryCredentialStore;
use fomkee_cli::workspace::model::CredentialPreference;
use fomkee_cli::workspace::service;
use fomkee_cli::workspace::store::{FileWorkspaceStore, WorkspaceStore};
use serde_json::json;
use std::fs;

use http_support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_environment_token_overrides_saved_default_and_uses_session_workspace() {
    // Arrange
    let server = ApiServer::start().await;
    let process = Process::new();
    process.save_profile("personal", "https://unused.example");

    // Act
    let output = process
        .authorized(&server)
        .args(["monitor", "list"])
        .output()
        .unwrap();

    // Assert
    assert_success(&output, json!({"items": [], "next_cursor": null}));
    server.finish(2).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_auth_status_reports_effective_origin_and_credential_source() {
    // Arrange
    let server = ApiServer::start().await;
    let process = Process::new();

    // Act
    let output = process
        .authorized(&server)
        .args(["auth", "status"])
        .output()
        .unwrap();

    // Assert
    assert_success(
        &output,
        json!({"workspace_id": WORKSPACE, "role": "ApiKey", "connection": {
            "alias": null, "api_url": server.origin, "credential_source": "environment"
        }}),
    );
    server.finish(1).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_explicit_alias_does_not_fall_back_to_environment_credentials() {
    // Arrange
    let server = ApiServer::start().await;
    let process = Process::new();

    // Act
    let output = process
        .authorized(&server)
        .args(["--workspace", "unknown", "auth", "status"])
        .output()
        .unwrap();

    // Assert
    assert_failure(&output, 2, "validation");
    server.finish(0).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_profile_environment_variable_selects_saved_credentials() {
    // Arrange
    let server = ApiServer::start().await;
    let process = Process::new();

    // Act
    let output = process
        .authorized(&server)
        .env("FOMKEE_PROFILE", "unknown")
        .args(["auth", "status"])
        .output()
        .unwrap();

    // Assert
    assert_failure(&output, 2, "validation");
    server.finish(0).await;
}

#[test]
fn test_workspace_flag_overrides_profile_environment_variable() {
    // Arrange
    let process = Process::new();
    process.save_profile("personal", "http://remote.example");

    // Act
    let output = process
        .json_command()
        .env("FOMKEE_PROFILE", "unknown")
        .args(["--workspace", "personal", "auth", "status"])
        .output()
        .unwrap();

    // Assert
    assert_failure(&output, 2, "validation");
    assert!(String::from_utf8_lossy(&output.stderr).contains("HTTPS origin"));
}

#[test]
fn test_saved_origin_is_not_replaced_by_environment_url() {
    // Arrange
    let process = Process::new();
    process.save_profile("personal", "http://remote.example");

    // Act
    let output = process
        .json_command()
        .env("FOMKEE_API_URL", "https://primary.fomkee.dev")
        .args(["auth", "status"])
        .output()
        .unwrap();

    // Assert
    assert_failure(&output, 2, "validation");
    assert!(String::from_utf8_lossy(&output.stderr).contains("HTTPS origin"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_invalid_api_token_is_rejected_without_saving_a_profile() {
    // Arrange
    let server = ApiServer::start().await;
    let process = Process::new();

    // Act
    let output = process.connect_stdin(&server, "fk_invalid_fixture");

    // Assert
    assert_failure(&output, 5, "api");
    assert!(!process.directory.path().join("workspaces.toml").exists());
    server.finish(1).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_http_discovery_persists_workspace_for_later_cli_invocations() {
    // Arrange
    let server = ApiServer::start().await;
    let process = Process::new();
    let credentials = InMemoryCredentialStore::default();

    // Act
    connect_over_http(&process, &server, &credentials).await;
    let output = process
        .json_command()
        .args(["workspace", "list"])
        .output()
        .unwrap();

    // Assert
    assert_success(
        &output,
        json!({"active": "personal", "workspaces": [{"alias": "personal", "active": true,
        "api_url": server.origin, "workspace_id": WORKSPACE, "name": "Personal", "slug": "personal", "credential_store": "keyring"}]}),
    );
    server.finish(2).await;
}

#[test]
fn test_workspace_use_persists_selection_without_network_or_keyring_access() {
    // Arrange
    let process = Process::new();
    process.save_profile("personal", "https://unused.example");

    // Act
    let output = process
        .json_command()
        .args(["workspace", "use", "personal"])
        .output()
        .unwrap();

    // Assert
    assert!(output.status.success(), "{:?}", output);
    let store = FileWorkspaceStore::open(process.directory.path().to_owned()).unwrap();
    assert_eq!(
        store.load().unwrap().active.unwrap().to_string(),
        "personal"
    );
}

#[test]
fn test_json_connect_without_token_fails_without_prompting() {
    // Arrange
    let process = Process::new();

    // Act
    let output = process
        .json_command()
        .args(["workspace", "connect", "personal"])
        .output()
        .unwrap();

    // Assert
    assert_failure(&output, 2, "validation");
}

#[test]
fn test_legacy_workspace_id_gives_migration_guidance() {
    // Arrange
    let process = Process::new();

    // Act
    let output = process
        .json_command()
        .env("FOMKEE_WORKSPACE_ID", WORKSPACE)
        .args(["auth", "status"])
        .output()
        .unwrap();

    // Assert
    assert_failure(&output, 2, "validation");
    assert!(String::from_utf8_lossy(&output.stderr).contains("unset it"));
}

async fn connect_over_http(
    process: &Process,
    server: &ApiServer,
    credentials: &InMemoryCredentialStore,
) {
    let config = server.config();
    let api = HttpFomkeeApi::new(&config).unwrap();
    let verified = service::verify(&api).await.unwrap();
    let store = FileWorkspaceStore::open(process.directory.path().to_owned()).unwrap();
    service::connect(
        verified,
        &store,
        &CredentialStores {
            keyring: credentials,
            file: credentials,
        },
        "personal".parse().unwrap(),
        config,
        CredentialPreference::Keyring,
    )
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_file_connection_authenticates_in_a_new_process_and_disconnects_locally() {
    // Arrange
    let server = ApiServer::start().await;
    let process = Process::new();
    assert!(process.connect_stdin(&server, TOKEN).status.success());

    // Act
    let output = process
        .json_command()
        .args(["auth", "status"])
        .output()
        .unwrap();
    let disconnected = process
        .json_command()
        .args(["workspace", "disconnect", "personal"])
        .output()
        .unwrap();

    // Assert
    assert_success(
        &output,
        json!({"workspace_id": WORKSPACE, "role": "ApiKey", "connection": {
            "alias": "personal", "api_url": server.origin, "credential_source": "file"
        }}),
    );
    assert_success(
        &disconnected,
        json!({"disconnected": "personal", "active": null, "api_key_revoked": false}),
    );
    assert!(
        !fs::read_to_string(process.directory.path().join("credentials.toml"))
            .unwrap()
            .contains(TOKEN)
    );
    server.finish(3).await;
}
