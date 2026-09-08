use dbus_secret_service_keyring_store::Store;
use keyring_core::Entry;

use crate::error::CliError;
use crate::workspace::credentials::entry_in;
use crate::workspace::model::CredentialId;

pub(crate) fn entry(id: &CredentialId) -> Result<Entry, CliError> {
    let store = Store::new().map_err(|_| CliError::CredentialStore)?;
    entry_in(store.as_ref(), id)
}
