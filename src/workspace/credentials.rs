use std::cell::RefCell;
use std::collections::BTreeMap;

use keyring_core::{Entry, Error as KeyringError, api::CredentialStoreApi};
mod platform;

use super::model::CredentialId;
use crate::config::ApiToken;
use crate::error::CliError;

/// Storage for workspace secrets, kept separate from profile metadata.
pub trait CredentialStore {
    /// Read the token or report that this backend cannot supply it.
    fn get(&self, id: &CredentialId) -> Result<ApiToken, CliError>;
    /// Store the token under its opaque local credential identifier.
    fn set(&self, id: &CredentialId, token: &ApiToken) -> Result<(), CliError>;
    /// Remove the local credential; an already absent entry is successful.
    fn delete(&self, id: &CredentialId) -> Result<(), CliError>;
}

/// macOS Keychain, Windows Credential Manager, or Linux Secret Service adapter.
pub struct OsCredentialStore;

fn entry(id: &CredentialId) -> Result<Entry, CliError> {
    platform::entry(id)
}

fn entry_in(store: &impl CredentialStoreApi, id: &CredentialId) -> Result<Entry, CliError> {
    store
        .build("fomkeecli", &id.to_string(), None)
        .map_err(|_| CliError::CredentialStore)
}

impl CredentialStore for OsCredentialStore {
    fn get(&self, id: &CredentialId) -> Result<ApiToken, CliError> {
        let token = entry(id)?.get_password().map_err(|error| {
            // KeyringError is foreign and non-exhaustive; only NoEntry is recoverable as a missing token.
            if matches!(error, KeyringError::NoEntry) {
                CliError::MissingSavedCredential
            } else {
                CliError::CredentialStore
            }
        })?;
        ApiToken::new(token)
    }

    fn set(&self, id: &CredentialId, token: &ApiToken) -> Result<(), CliError> {
        entry(id)?
            .set_password(token.expose())
            .map_err(|_| CliError::CredentialStore)
    }

    fn delete(&self, id: &CredentialId) -> Result<(), CliError> {
        match entry(id)?.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(_) => Err(CliError::CredentialStore),
        }
    }
}

/// Reusable in-memory credential store for isolated command tests.
#[derive(Default)]
pub struct InMemoryCredentialStore {
    tokens: RefCell<BTreeMap<CredentialId, String>>,
}

impl CredentialStore for InMemoryCredentialStore {
    fn get(&self, id: &CredentialId) -> Result<ApiToken, CliError> {
        let tokens = self
            .tokens
            .try_borrow()
            .map_err(|_| CliError::CredentialStore)?;
        ApiToken::new(
            tokens
                .get(id)
                .ok_or(CliError::MissingSavedCredential)?
                .clone(),
        )
    }

    fn set(&self, id: &CredentialId, token: &ApiToken) -> Result<(), CliError> {
        self.tokens
            .try_borrow_mut()
            .map_err(|_| CliError::CredentialStore)?
            .insert(id.clone(), token.expose().to_owned());
        Ok(())
    }

    fn delete(&self, id: &CredentialId) -> Result<(), CliError> {
        self.tokens
            .try_borrow_mut()
            .map_err(|_| CliError::CredentialStore)?
            .remove(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyring_core::mock;

    #[test]
    fn test_current_keyring_keeps_the_existing_service_and_credential_id() {
        // Arrange
        let store = mock::Store::new().unwrap();
        let id = CredentialId::generate().unwrap();

        // Act
        let entry = entry_in(store.as_ref(), &id).unwrap();

        // Assert
        assert_eq!(
            entry.get_specifiers(),
            Some(("fomkeecli".into(), id.to_string()))
        );
    }
}
