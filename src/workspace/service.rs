use super::output::{Disconnected, ProfileOutput, WorkspaceList};
use crate::dto::{Session, Workspace};
use crate::wire::Response;

use super::credential_stores::CredentialStores;
use super::credentials::CredentialStore;
use super::model::{
    CredentialBackend, CredentialId, CredentialPreference, WorkspaceAlias, WorkspaceProfile,
    WorkspaceRegistry,
};
use super::store::WorkspaceStore;
use crate::client::FomkeeApi;
use crate::config::{Config, api_url};
use crate::error::CliError;
use crate::model::WorkspaceId;

/// Identity checked against both public discovery endpoints.
pub struct VerifiedWorkspace {
    workspace: Workspace,
}

/// Resolve remote identity before acquiring a local configuration lock.
pub async fn verify(api: &impl FomkeeApi) -> Result<VerifiedWorkspace, CliError> {
    let session = api.session().await?;
    let workspace_id = session.data().workspace_id;
    let workspace = api.workspace(&workspace_id).await?;
    if workspace.data().id != workspace_id {
        return Err(CliError::MalformedResponse(
            "workspace response does not match token identity".into(),
        ));
    }
    Ok(VerifiedWorkspace {
        workspace: workspace.data().clone(),
    })
}

/// Commit a previously verified identity under the caller's configuration lock.
pub fn connect(
    verified: VerifiedWorkspace,
    store: &impl WorkspaceStore,
    credentials: &CredentialStores<'_, impl CredentialStore, impl CredentialStore>,
    alias: WorkspaceAlias,
    config: Config,
    preference: CredentialPreference,
) -> Result<ProfileOutput, CliError> {
    let mut registry = store.load()?;
    if registry.workspaces.contains_key(&alias) {
        return Err(CliError::InvalidInput(
            "alias is already connected; disconnect it first or choose another alias".into(),
        ));
    }
    let workspace = verified.workspace;
    let workspace_id = workspace.id;
    let credential_id = CredentialId::generate()?;
    let credential_store = credentials.save_new(preference, &credential_id, &config.token)?;
    let profile = WorkspaceProfile {
        api_url: config.base_url.to_string(),
        workspace_id,
        name: workspace.name,
        credential_id,
        credential_store,
    };
    registry.workspaces.insert(alias.clone(), profile.clone());
    if registry.active.is_none() {
        registry.active = Some(alias.clone());
    }
    if let Err(error) = store.save(&registry) {
        credentials.delete(profile.credential_store, &profile.credential_id).map_err(|_| CliError::Configuration(
            format!("profile save and credential cleanup failed; remove orphaned credential {} from {:?} storage", profile.credential_id, profile.credential_store)
        ))?;
        return Err(error);
    }
    Ok(profile_output(
        &alias,
        &profile,
        registry.active.as_ref() == Some(&alias),
    ))
}

/// List locally connected workspaces without loading secrets or calling the API.
pub fn list(store: &impl WorkspaceStore) -> Result<WorkspaceList, CliError> {
    Ok(list_registry(store.load()?))
}

/// Present a metadata snapshot without locking or touching credentials.
pub fn list_registry(registry: WorkspaceRegistry) -> WorkspaceList {
    let workspaces: Vec<ProfileOutput> = registry
        .workspaces
        .iter()
        .map(|(alias, profile)| {
            profile_output(alias, profile, registry.active.as_ref() == Some(alias))
        })
        .collect();
    WorkspaceList {
        active: registry.active,
        workspaces,
    }
}

/// Select the default workspace for subsequent CLI invocations.
pub fn use_workspace(
    store: &impl WorkspaceStore,
    alias: &WorkspaceAlias,
) -> Result<ProfileOutput, CliError> {
    let mut registry = store.load()?;
    let profile = registry
        .workspaces
        .get(alias)
        .ok_or_else(unknown_workspace)?
        .clone();
    registry.active = Some(alias.clone());
    store.save(&registry)?;
    Ok(profile_output(alias, &profile, true))
}

/// Forget a local workspace and its saved token; does not revoke the remote API key.
pub fn disconnect(
    store: &impl WorkspaceStore,
    credentials: &CredentialStores<'_, impl CredentialStore, impl CredentialStore>,
    alias: &WorkspaceAlias,
) -> Result<Disconnected, CliError> {
    let mut registry = store.load()?;
    let profile = registry
        .workspaces
        .get(alias)
        .ok_or_else(unknown_workspace)?;
    credentials.delete(profile.credential_store, &profile.credential_id)?;
    registry.workspaces.remove(alias);
    if registry.active.as_ref() == Some(alias) {
        registry.active = None;
    }
    store.save(&registry)?;
    Ok(Disconnected {
        disconnected: alias.clone(),
        active: registry.active,
        api_key_revoked: false,
    })
}

/// Resolve an explicit alias or the locally selected default.
pub fn selected_profile(
    store: &impl WorkspaceStore,
    alias: Option<&WorkspaceAlias>,
) -> Result<(WorkspaceAlias, WorkspaceProfile), CliError> {
    let registry = store.load()?;
    select_profile(&registry, alias)
}

/// Select connection metadata without reading its credential.
pub fn select_profile(
    registry: &WorkspaceRegistry,
    alias: Option<&WorkspaceAlias>,
) -> Result<(WorkspaceAlias, WorkspaceProfile), CliError> {
    let alias = alias
        .or(registry.active.as_ref())
        .ok_or(CliError::MissingCredentials)?;
    let profile = registry
        .workspaces
        .get(alias)
        .ok_or_else(unknown_workspace)?;
    Ok((alias.clone(), profile.clone()))
}

/// Load the saved API origin and credential as one connection.
pub fn profile_config(
    profile: &WorkspaceProfile,
    credentials: &CredentialStores<'_, impl CredentialStore, impl CredentialStore>,
) -> Result<Config, CliError> {
    Ok(Config {
        base_url: api_url(&profile.api_url)?,
        token: credentials.get(profile.credential_store, &profile.credential_id)?,
    })
}

/// Verify the current server identity before using a cached workspace ID.
pub fn session_workspace(session: &Response<Session>) -> Result<WorkspaceId, CliError> {
    Ok(session.data().workspace_id)
}

fn unknown_workspace() -> CliError {
    CliError::InvalidInput(
        "workspace alias is not connected; run workspace list or workspace connect".into(),
    )
}

fn profile_output(
    alias: &WorkspaceAlias,
    profile: &WorkspaceProfile,
    active: bool,
) -> ProfileOutput {
    ProfileOutput {
        alias: alias.clone(),
        api_url: profile.api_url.clone(),
        workspace_id: profile.workspace_id,
        name: profile.name.clone(),
        active,
        credential_store: profile.credential_store,
        credential_storage_notice: (profile.credential_store == CredentialBackend::File).then(
            || "Token stored unencrypted in the protected local credentials.toml file.".into(),
        ),
    }
}
