use crate::error::CliError;
use crate::workspace::model::CredentialId;
use keyring_core::Entry;

pub(crate) fn entry(_: &CredentialId) -> Result<Entry, CliError> {
    Err(CliError::CredentialStore)
}
