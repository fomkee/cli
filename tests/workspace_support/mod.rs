use std::cell::RefCell;
use std::fs;
use std::path::Path;

use fomkee_cli::client::InMemoryFomkeeApi;
use fomkee_cli::config::{ApiToken, Config, DEFAULT_API_URL, api_url};
use fomkee_cli::error::CliError;
use fomkee_cli::workspace::credential_stores::CredentialStores;
use fomkee_cli::workspace::credentials::{CredentialStore, InMemoryCredentialStore};
use fomkee_cli::workspace::model::{
    CredentialBackend, CredentialId, CredentialPreference, WorkspaceAlias, WorkspaceProfile,
    WorkspaceRegistry,
};
use fomkee_cli::workspace::service;
use fomkee_cli::workspace::store::{FileWorkspaceStore, InMemoryWorkspaceStore, WorkspaceStore};
use fomkee_cli::workspace::{Connection, ConnectionOrigin};
use serde_json::{Value, json};

pub const FIRST: &str = "00000000-0000-0000-0000-000000000001";
pub const SECOND: &str = "00000000-0000-0000-0000-000000000002";
pub const TOKEN: &str = "fk_fixture_only_not_a_real_credential";

pub fn alias(name: &str) -> WorkspaceAlias {
    name.parse().unwrap()
}

pub fn config() -> Config {
    Config {
        base_url: api_url(DEFAULT_API_URL).unwrap(),
        token: ApiToken::new(TOKEN.into()).unwrap(),
    }
}

#[derive(Default)]
pub struct Scenario {
    pub store: InMemoryWorkspaceStore,
    pub credentials: InMemoryCredentialStore,
}

impl Scenario {
    pub fn stores(&self) -> CredentialStores<'_, InMemoryCredentialStore, UnavailableCredentials> {
        CredentialStores {
            keyring: &self.credentials,
            file: &UnavailableCredentials,
        }
    }

    pub async fn connect(&self, name: &str, id: &str) -> Result<Value, CliError> {
        service::connect(
            service::verify(&InMemoryFomkeeApi::with_session(id.parse().map_err(
                |_| CliError::MalformedResponse("invalid fixture identity".into()),
            )?))
            .await?,
            &self.store,
            &self.stores(),
            alias(name),
            config(),
            CredentialPreference::Keyring,
        )
        .map(|value| serde_json::to_value(value).unwrap())
    }
}

pub async fn two_workspaces() -> Scenario {
    let scenario = Scenario::default();
    scenario.connect("personal", FIRST).await.unwrap();
    scenario.connect("company", SECOND).await.unwrap();
    scenario
}

pub fn expected_connection(alias: &str, id: &str, active: bool) -> Value {
    json!({"alias": alias, "api_url": "https://primary.fomkee.com/", "workspace_id": id,
        "name": "Test workspace", "active": active, "credential_store": "keyring"})
}

pub fn assert_saved_connection(scenario: &Scenario, name: &str, id: &str) {
    let (_, profile) = service::selected_profile(&scenario.store, Some(&alias(name))).unwrap();
    let config = service::profile_config(&profile, &scenario.stores()).unwrap();
    assert_eq!(profile.workspace_id.to_string(), id);
    assert_eq!(config.base_url.as_str(), "https://primary.fomkee.com/");
    assert!(
        !serde_json::to_string(&scenario.store.load().unwrap())
            .unwrap()
            .contains(TOKEN)
    );
}

pub fn assert_disconnected(scenario: &Scenario, id: &CredentialId) {
    assert!(matches!(
        scenario.credentials.get(id),
        Err(CliError::MissingSavedCredential)
    ));
    assert_eq!(
        serde_json::to_value(service::list(&scenario.store).unwrap()).unwrap(),
        json!({"active": null, "workspaces": [expected_connection("company", SECOND, false)]})
    );
}

pub struct UnavailableCredentials;
impl CredentialStore for UnavailableCredentials {
    fn get(&self, _: &CredentialId) -> Result<ApiToken, CliError> {
        Err(CliError::CredentialStore)
    }
    fn set(&self, _: &CredentialId, _: &ApiToken) -> Result<(), CliError> {
        Err(CliError::CredentialStore)
    }
    fn delete(&self, _: &CredentialId) -> Result<(), CliError> {
        Err(CliError::CredentialStore)
    }
}

pub struct UnwritableStore;
impl WorkspaceStore for UnwritableStore {
    fn load(&self) -> Result<WorkspaceRegistry, CliError> {
        Ok(WorkspaceRegistry::default())
    }
    fn save(&self, _: &WorkspaceRegistry) -> Result<(), CliError> {
        Err(CliError::Configuration("fixture write failure".into()))
    }
}

#[derive(Default)]
pub struct RecordingCredentials {
    store: InMemoryCredentialStore,
    created: RefCell<Option<CredentialId>>,
}

impl CredentialStore for RecordingCredentials {
    fn get(&self, id: &CredentialId) -> Result<ApiToken, CliError> {
        self.store.get(id)
    }
    fn set(&self, id: &CredentialId, token: &ApiToken) -> Result<(), CliError> {
        *self.created.borrow_mut() = Some(id.clone());
        self.store.set(id, token)
    }
    fn delete(&self, id: &CredentialId) -> Result<(), CliError> {
        self.store.delete(id)
    }
}

pub async fn connect_with(
    store: &impl WorkspaceStore,
    credentials: &impl CredentialStore,
) -> Result<Value, CliError> {
    service::connect(
        service::verify(&InMemoryFomkeeApi::with_session(FIRST.parse().unwrap())).await?,
        store,
        &CredentialStores {
            keyring: credentials,
            file: &UnavailableCredentials,
        },
        alias("personal"),
        config(),
        CredentialPreference::Keyring,
    )
    .map(|value| serde_json::to_value(value).unwrap())
}

pub fn assert_credential_rolled_back(credentials: &RecordingCredentials) {
    let id = credentials.created.borrow().clone().unwrap();
    assert!(matches!(
        credentials.get(&id),
        Err(CliError::MissingSavedCredential)
    ));
}

fn registry() -> WorkspaceRegistry {
    let mut registry = WorkspaceRegistry::default();
    registry.workspaces.insert(
        alias("personal"),
        WorkspaceProfile {
            api_url: DEFAULT_API_URL.into(),
            workspace_id: FIRST.parse().unwrap(),
            name: "Personal".into(),
            credential_id: CredentialId::generate().unwrap(),
            credential_store: CredentialBackend::Keyring,
        },
    );
    registry.active = Some(alias("personal"));
    registry
}

pub fn assert_store_contract(store: &impl WorkspaceStore) {
    assert!(store.load().unwrap().workspaces.is_empty());
    let expected = registry();
    store.save(&expected).unwrap();
    assert_eq!(
        serde_json::to_value(store.load().unwrap()).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
    store.save(&WorkspaceRegistry::default()).unwrap();
    assert!(store.load().unwrap().workspaces.is_empty());
}

pub fn save_fixture_registry(directory: &Path) {
    let store = FileWorkspaceStore::open(directory.to_owned()).unwrap();
    store.save(&registry()).unwrap();
}

pub fn save_legacy_registry(directory: &Path) {
    let mut legacy = serde_json::to_value(registry()).unwrap();
    legacy
        .get_mut("workspaces")
        .unwrap()
        .get_mut("personal")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("credential_store");
    fs::write(
        directory.join("workspaces.json"),
        serde_json::to_vec_pretty(&legacy).unwrap(),
    )
    .unwrap();
}

pub fn assert_metadata_has_no_secret(directory: &Path) {
    let contents = fs::read_to_string(directory.join("workspaces.toml")).unwrap();
    assert!(!contents.contains(TOKEN));
    assert!(!contents.contains("token"));
}

pub fn saved_connection(id: &str) -> Connection {
    Connection {
        config: config(),
        origin: ConnectionOrigin::Saved {
            alias: alias("personal"),
            workspace_id: id.parse().unwrap(),
            credential_store: CredentialBackend::Keyring,
            name: "Personal".into(),
        },
    }
}
