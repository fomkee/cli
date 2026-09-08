#![cfg(test)]

use fomkee_cli::config::ApiToken;
use fomkee_cli::error::CliError;
use fomkee_cli::workspace::credential_stores::CredentialStores;
use fomkee_cli::workspace::credentials::{CredentialStore, InMemoryCredentialStore};
use fomkee_cli::workspace::file_credentials::FileCredentialStore;
use fomkee_cli::workspace::model::{CredentialBackend, CredentialId, CredentialPreference};
use std::fs;
use tempfile::{TempDir, tempdir};

const TOKEN: &str = "fk_file_fixture_only";

struct Unavailable;

impl CredentialStore for Unavailable {
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

fn token() -> ApiToken {
    ApiToken::new(TOKEN.into()).unwrap()
}

fn fixture() -> (TempDir, FileCredentialStore, CredentialId) {
    let directory = tempdir().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
    }
    let store = FileCredentialStore::new(directory.path().to_owned());
    (directory, store, CredentialId::generate().unwrap())
}

fn assert_store_contract(store: &impl CredentialStore) {
    let first = CredentialId::generate().unwrap();
    let second = CredentialId::generate().unwrap();
    assert!(matches!(
        store.get(&first),
        Err(CliError::MissingSavedCredential)
    ));
    store.set(&first, &token()).unwrap();
    store.set(&second, &token()).unwrap();
    store
        .set(&first, &ApiToken::new("fk_replacement".into()).unwrap())
        .unwrap();
    assert!(store.get(&first).is_ok());
    store.delete(&first).unwrap();
    store.delete(&first).unwrap();
    assert!(matches!(
        store.get(&first),
        Err(CliError::MissingSavedCredential)
    ));
    assert!(store.get(&second).is_ok());
}

#[test]
fn test_file_credentials_satisfy_store_contract() {
    // Arrange
    let (_directory, store, _) = fixture();

    // Act / Assert
    assert_store_contract(&store);
}

#[test]
fn test_memory_credentials_satisfy_store_contract() {
    // Arrange
    let store = InMemoryCredentialStore::default();

    // Act / Assert
    assert_store_contract(&store);
}

#[test]
fn test_auto_prefers_keyring_without_touching_file_store() {
    // Arrange
    let keyring = InMemoryCredentialStore::default();
    let stores = CredentialStores {
        keyring: &keyring,
        file: &Unavailable,
    };

    // Act
    let backend = stores
        .save_new(
            CredentialPreference::Auto,
            &CredentialId::generate().unwrap(),
            &token(),
        )
        .unwrap();

    // Assert
    assert_eq!(backend, CredentialBackend::Keyring);
}

#[test]
fn test_auto_saves_to_file_when_keyring_is_unavailable() {
    // Arrange
    let (_directory, file, id) = fixture();
    let stores = CredentialStores {
        keyring: &Unavailable,
        file: &file,
    };

    // Act
    let backend = stores
        .save_new(CredentialPreference::Auto, &id, &token())
        .unwrap();

    // Assert
    assert_eq!(backend, CredentialBackend::File);
    assert!(file.get(&id).is_ok());
}

#[test]
fn test_explicit_keyring_does_not_fall_back() {
    // Arrange
    let (directory, file, id) = fixture();
    let stores = CredentialStores {
        keyring: &Unavailable,
        file: &file,
    };

    // Act
    let result = stores.save_new(CredentialPreference::Keyring, &id, &token());

    // Assert
    assert!(matches!(result, Err(CliError::CredentialStore)));
    assert!(!directory.path().join("credentials.toml").exists());
}

#[test]
fn test_explicit_file_bypasses_keyring() {
    // Arrange
    let (_directory, file, id) = fixture();
    let stores = CredentialStores {
        keyring: &Unavailable,
        file: &file,
    };

    // Act
    let backend = stores
        .save_new(CredentialPreference::File, &id, &token())
        .unwrap();

    // Assert
    assert_eq!(backend, CredentialBackend::File);
}

#[test]
fn test_keyring_read_failure_does_not_read_fallback_token() {
    // Arrange
    let (_directory, file, id) = fixture();
    file.set(&id, &token()).unwrap();
    let stores = CredentialStores {
        keyring: &Unavailable,
        file: &file,
    };

    // Act
    let result = stores.get(CredentialBackend::Keyring, &id);

    // Assert
    assert!(matches!(result, Err(CliError::CredentialStore)));
}

#[test]
fn test_missing_keyring_entry_does_not_read_fallback_token() {
    // Arrange
    let (_directory, file, id) = fixture();
    file.set(&id, &token()).unwrap();
    let keyring = InMemoryCredentialStore::default();
    let stores = CredentialStores {
        keyring: &keyring,
        file: &file,
    };

    // Act
    let result = stores.get(CredentialBackend::Keyring, &id);

    // Assert
    assert!(matches!(result, Err(CliError::MissingSavedCredential)));
}

#[test]
fn test_file_delete_does_not_require_a_working_keyring() {
    // Arrange
    let (_directory, file, id) = fixture();
    file.set(&id, &token()).unwrap();
    let stores = CredentialStores {
        keyring: &Unavailable,
        file: &file,
    };

    // Act
    stores.delete(CredentialBackend::File, &id).unwrap();

    // Assert
    assert!(matches!(
        file.get(&id),
        Err(CliError::MissingSavedCredential)
    ));
}

#[test]
fn test_file_is_valid_toml_and_survives_reopening() {
    // Arrange
    let (directory, store, id) = fixture();
    store.set(&id, &token()).unwrap();

    // Act
    let reopened = FileCredentialStore::new(directory.path().to_owned());
    let contents = fs::read_to_string(directory.path().join("credentials.toml")).unwrap();

    // Assert
    assert!(reopened.get(&id).is_ok());
    assert_eq!(
        toml::from_str::<toml::Value>(&contents)
            .unwrap()
            .get("tokens")
            .unwrap()
            .get(id.to_string())
            .unwrap()
            .as_str(),
        Some(TOKEN)
    );
}

#[test]
fn test_malformed_secret_file_is_not_overwritten_or_leaked() {
    // Arrange
    let (directory, store, id) = fixture();
    store.set(&id, &token()).unwrap();
    let path = directory.path().join("credentials.toml");
    let malformed = format!("[tokens]\n\"{id}\" = \"{TOKEN}");
    fs::write(&path, &malformed).unwrap();

    // Act
    let error = store.set(&id, &token()).unwrap_err();

    // Assert
    assert!(!error.to_string().contains(TOKEN));
    assert_eq!(fs::read_to_string(path).unwrap(), malformed);
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};

    #[test]
    fn test_new_secret_file_is_owner_only() {
        // Arrange
        let (directory, store, id) = fixture();

        // Act
        store.set(&id, &token()).unwrap();

        // Assert
        assert_eq!(
            fs::metadata(directory.path().join("credentials.toml"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    #[test]
    fn test_shared_secret_file_is_rejected() {
        // Arrange
        let (directory, store, id) = fixture();
        store.set(&id, &token()).unwrap();
        fs::set_permissions(
            directory.path().join("credentials.toml"),
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();

        // Act
        let result = store.get(&id);

        // Assert
        assert!(matches!(result, Err(CliError::Configuration(_))));
    }

    #[test]
    fn test_symlink_secret_file_is_rejected() {
        // Arrange
        let (directory, store, id) = fixture();
        let target = directory.path().join("unrelated.toml");
        fs::write(&target, "unrelated").unwrap();
        symlink(&target, directory.path().join("credentials.toml")).unwrap();

        // Act
        let result = store.set(&id, &token());

        // Assert
        assert!(matches!(result, Err(CliError::Configuration(_))));
        assert_eq!(fs::read_to_string(target).unwrap(), "unrelated");
    }

    #[test]
    fn test_shared_writable_directory_is_rejected() {
        // Arrange
        let (directory, store, id) = fixture();
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o777)).unwrap();

        // Act
        let result = store.set(&id, &token());

        // Assert
        assert!(matches!(result, Err(CliError::Configuration(_))));
    }
}
