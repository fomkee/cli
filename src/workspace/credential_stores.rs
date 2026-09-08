use super::credentials::CredentialStore;
use super::model::{CredentialBackend, CredentialId, CredentialPreference};
use crate::config::ApiToken;
use crate::error::CliError;

/// Routes credentials to their recorded backend; fallback applies only to new writes.
pub struct CredentialStores<'a, K, F> {
    pub keyring: &'a K,
    pub file: &'a F,
}

impl<K: CredentialStore, F: CredentialStore> CredentialStores<'_, K, F> {
    pub fn save_new(
        &self,
        preference: CredentialPreference,
        id: &CredentialId,
        token: &ApiToken,
    ) -> Result<CredentialBackend, CliError> {
        match preference {
            CredentialPreference::File => {
                self.file.set(id, token)?;
                Ok(CredentialBackend::File)
            }
            CredentialPreference::Keyring => {
                self.keyring.set(id, token)?;
                Ok(CredentialBackend::Keyring)
            }
            CredentialPreference::Auto => match self.keyring.set(id, token) {
                Ok(()) => Ok(CredentialBackend::Keyring),
                Err(CliError::CredentialStore) => {
                    self.file.set(id, token)?;
                    Ok(CredentialBackend::File)
                }
                Err(error) => Err(error),
            },
        }
    }

    pub fn get(&self, backend: CredentialBackend, id: &CredentialId) -> Result<ApiToken, CliError> {
        match backend {
            CredentialBackend::Keyring => self.keyring.get(id),
            CredentialBackend::File => self.file.get(id),
        }
    }

    pub fn delete(&self, backend: CredentialBackend, id: &CredentialId) -> Result<(), CliError> {
        match backend {
            CredentialBackend::Keyring => self.keyring.delete(id),
            CredentialBackend::File => self.file.delete(id),
        }
    }
}
