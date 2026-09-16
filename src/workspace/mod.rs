pub mod credential_stores;
pub mod credentials;
pub mod file_credentials;
pub mod model;
pub mod output;
mod private_files;
pub mod service;
pub mod store;

use std::env;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

use crate::dto::Session;
use crate::result::{ActionResult, CommandResult, WorkspaceAction};
use crate::wire::Response;
use clap::Subcommand;
use output::{AuthStatus, ConnectionInfo, CredentialSource};

use crate::client::HttpFomkeeApi;
use crate::config::{ApiToken, Config, DEFAULT_API_URL, api_url};
use crate::error::CliError;
use crate::model::WorkspaceId;
use crate::output::OutputMode;
use credential_stores::CredentialStores;
use credentials::OsCredentialStore;
use file_credentials::FileCredentialStore;
use model::{CredentialBackend, CredentialPreference, WorkspaceAlias};
use store::FileWorkspaceStore;

/// Commands for local workspace connections, without remote workspace mutations.
#[derive(Debug, Subcommand)]
pub(crate) enum WorkspaceCommand {
    /// Connect a workspace using an API token.
    Connect {
        /// Name to save this connection under.
        alias: WorkspaceAlias,
        /// Read the token from stdin instead of a hidden terminal prompt.
        #[arg(long)]
        token_stdin: bool,
        /// Fomkee server address for this workspace.
        #[arg(long, env = "FOMKEE_API_URL", default_value = DEFAULT_API_URL)]
        api_url: String,
        /// Where to save the token; auto prefers the keyring, with a local file fallback.
        #[arg(long, value_enum, default_value = "auto")]
        credential_store: CredentialPreference,
    },
    /// List your saved workspace connections.
    List,
    /// Select the default workspace.
    Use {
        /// Saved workspace alias shown by workspace list.
        alias: WorkspaceAlias,
    },
    /// Forget a saved connection; its API token remains valid.
    Disconnect {
        /// Saved workspace alias shown by workspace list.
        alias: WorkspaceAlias,
    },
}

/// Execute a local workspace operation.
pub(crate) async fn execute(
    command: WorkspaceCommand,
    output: Option<OutputMode>,
    directory: PathBuf,
) -> Result<CommandResult, CliError> {
    match command {
        WorkspaceCommand::List => Ok(CommandResult::WorkspaceList(service::list_registry(
            store::read_registry(&directory)?,
        ))),
        WorkspaceCommand::Connect {
            alias,
            token_stdin,
            api_url: origin,
            credential_store,
        } => {
            connect(
                alias,
                token_stdin,
                &origin,
                credential_store,
                output,
                directory,
            )
            .await
        }
        WorkspaceCommand::Use { alias } => {
            let store = FileWorkspaceStore::open(directory)?;
            service::use_workspace(&store, &alias).map(|result| {
                CommandResult::WorkspaceAction(ActionResult {
                    action: WorkspaceAction::Selected,
                    result,
                })
            })
        }
        WorkspaceCommand::Disconnect { alias } => {
            let store = FileWorkspaceStore::open(directory)?;
            let file = FileCredentialStore::new(store.directory().to_owned());
            service::disconnect(
                &store,
                &CredentialStores {
                    keyring: &OsCredentialStore,
                    file: &file,
                },
                &alias,
            )
            .map(CommandResult::Disconnected)
        }
    }
}

async fn connect(
    alias: WorkspaceAlias,
    token_stdin: bool,
    origin: &str,
    preference: CredentialPreference,
    output: Option<OutputMode>,
    directory: PathBuf,
) -> Result<CommandResult, CliError> {
    let config = Config {
        base_url: api_url(origin)?,
        token: read_token(token_stdin, output)?,
    };
    let api = HttpFomkeeApi::new(&config)?;
    let verified = service::verify(&api).await?;
    let store = FileWorkspaceStore::open(directory)?;
    let file = FileCredentialStore::new(store.directory().to_owned());
    let credentials = CredentialStores {
        keyring: &OsCredentialStore,
        file: &file,
    };
    let result = service::connect(verified, &store, &credentials, alias, config, preference)?;
    Ok(CommandResult::WorkspaceAction(ActionResult {
        action: WorkspaceAction::Connected,
        result,
    }))
}

fn read_token(from_stdin: bool, output: Option<OutputMode>) -> Result<ApiToken, CliError> {
    let value = if from_stdin {
        let mut value = String::new();
        io::stdin()
            .take(4097)
            .read_to_string(&mut value)
            .map_err(|_| CliError::InvalidInput("cannot read token from stdin".into()))?;
        value
    } else if let Ok(value) = env::var("FOMKEE_API_TOKEN") {
        value
    } else if !matches!(output, Some(OutputMode::Json))
        && io::stdin().is_terminal()
        && io::stderr().is_terminal()
    {
        rpassword::prompt_password("Workspace API token: ")
            .map_err(|_| CliError::InvalidInput("cannot read token from terminal".into()))?
    } else {
        return Err(CliError::InvalidInput("connect needs --token-stdin, FOMKEE_API_TOKEN, or an interactive terminal (explicit --output json never prompts)".into()));
    };
    if value.len() > 4096 {
        return Err(CliError::InvalidInput("API token is too long".into()));
    }
    ApiToken::new(value)
}

/// Resolved workspace identity and diagnostic metadata for one invocation.
pub struct Connection {
    pub config: Config,
    pub origin: ConnectionOrigin,
}

/// Saved connections carry their complete identity; environment connections do not.
pub enum ConnectionOrigin {
    Environment,
    Saved {
        alias: WorkspaceAlias,
        workspace_id: WorkspaceId,
        credential_store: CredentialBackend,
        name: String,
    },
}

/// Explicit aliases win over environment tokens; environment tokens win over the saved default.
pub fn resolve(alias: Option<&WorkspaceAlias>, directory: PathBuf) -> Result<Connection, CliError> {
    if env::var_os("FOMKEE_WORKSPACE_ID").is_some() {
        return Err(CliError::InvalidInput("FOMKEE_WORKSPACE_ID is no longer supported; unset it and select a saved alias with --workspace or FOMKEE_PROFILE".into()));
    }
    if alias.is_none() && env::var_os("FOMKEE_API_TOKEN").is_some() {
        return Ok(Connection {
            config: Config::from_environment()?,
            origin: ConnectionOrigin::Environment,
        });
    }
    let store = FileWorkspaceStore::open(directory)?;
    let (alias, profile) = service::selected_profile(&store, alias)?;
    let file = FileCredentialStore::new(store.directory().to_owned());
    let credentials = CredentialStores {
        keyring: &OsCredentialStore,
        file: &file,
    };
    Ok(Connection {
        config: service::profile_config(&profile, &credentials)?,
        origin: ConnectionOrigin::Saved {
            alias,
            workspace_id: profile.workspace_id,
            credential_store: profile.credential_store,
            name: profile.name,
        },
    })
}

impl Connection {
    /// Reject a saved profile whose token now belongs to a different workspace.
    pub fn verify_session(&self, session: &Response<Session>) -> Result<WorkspaceId, CliError> {
        let actual = session.data().workspace_id;
        if let ConnectionOrigin::Saved { workspace_id, .. } = self.origin
            && workspace_id != actual
        {
            return Err(CliError::InvalidInput(
                "saved workspace does not match token identity; disconnect and reconnect it".into(),
            ));
        }
        Ok(actual)
    }

    pub(crate) fn workspace_name(&self) -> Option<&str> {
        match &self.origin {
            ConnectionOrigin::Environment => None,
            ConnectionOrigin::Saved { name, .. } => Some(name),
        }
    }

    /// Enrich the original session result with non-secret connection metadata.
    pub fn status(&self, session: Response<Session>) -> AuthStatus {
        let (alias, credential_source) = match &self.origin {
            ConnectionOrigin::Environment => (None, CredentialSource::Environment),
            ConnectionOrigin::Saved {
                alias,
                credential_store,
                ..
            } => (
                Some(alias.clone()),
                CredentialSource::from(*credential_store),
            ),
        };
        AuthStatus {
            session,
            connection: ConnectionInfo {
                alias,
                api_url: self.config.base_url.to_string(),
                credential_source,
            },
        }
    }
}
