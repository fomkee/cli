use crate::dto::update::UpdateInput;
use std::time::Duration;

use crate::dto::dry_run::DryRun;
use crate::dto::{CreatedMonitor, Entitlements, Monitor, MonitorPage, Session, Workspace};
use crate::error::response::{ResponseCause, ResponseFailure, ResponseOutcome};
use crate::error::transport::TransportFailure;
use crate::wire::Response;
use async_trait::async_trait;
use reqwest::redirect::Policy;
use reqwest::{Method, StatusCode, header};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use url::{Url, form_urlencoded};

use crate::config::Config;
use crate::error::CliError;
use crate::model::{MonitorId, WorkspaceId};
use crate::monitoring::{CreateInput, DryRunInput, LifecycleInput};

const USER_AGENT: &str = concat!("fomkee-cli/", env!("CARGO_PKG_VERSION"));
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const PAGE_SIZE: u32 = 100;
const MAX_RETRY_AFTER: Duration = Duration::from_secs(5);

use super::FomkeeApi;

/// Production HTTP adapter with bounded requests and no mutation retries.
pub struct HttpFomkeeApi {
    http: reqwest::Client,
    base_url: Url,
    token: String,
}
impl HttpFomkeeApi {
    /// Build an adapter for a validated API origin and token.
    pub fn new(config: &Config) -> Result<Self, CliError> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(REQUEST_TIMEOUT)
            .user_agent(USER_AGENT)
            .redirect(Policy::none())
            .build()
            .map_err(|error| CliError::Transport(TransportFailure::Http(error)))?;
        Ok(Self {
            http,
            base_url: config.base_url.clone(),
            token: config.token.expose().to_owned(),
        })
    }
    fn url(&self, path: &str) -> Result<Url, CliError> {
        self.base_url
            .join(path.trim_start_matches('/'))
            .map_err(|e| CliError::InvalidInput(format!("invalid API URL: {e}")))
    }
    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        mutation: bool,
    ) -> Result<Response<T>, CliError> {
        let url = self.url(path)?;
        let mut request = self.http.request(method, url).bearer_auth(&self.token);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.map_err(|error| {
            if mutation {
                CliError::OutcomeUnknown(TransportFailure::Http(error))
            } else {
                CliError::Transport(TransportFailure::Http(error))
            }
        })?;
        let status = response.status();
        let retry_after_secs = response
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let outcome = if mutation {
            if status.is_success() {
                ResponseOutcome::MutationAcknowledged
            } else {
                ResponseOutcome::MutationErrorReported
            }
        } else {
            ResponseOutcome::Read
        };
        let failure = |source| {
            CliError::Response(ResponseFailure {
                status: status.as_u16(),
                outcome,
                source,
                retry_after_secs,
            })
        };
        if status == StatusCode::NO_CONTENT {
            return Response::decode(Value::Null)
                .map_err(|error| failure(ResponseCause::Decode(error)));
        }
        let text = response
            .text()
            .await
            .map_err(|error| failure(ResponseCause::Body(error)))?;
        if !status.is_success() {
            return Err(api_error(status, &text, retry_after_secs, outcome));
        }
        serde_json::from_str(&text)
            .and_then(Response::decode)
            .map_err(|error| failure(ResponseCause::Decode(error)))
    }
    async fn read_request<T: DeserializeOwned>(&self, path: &str) -> Result<Response<T>, CliError> {
        let first = self.request(Method::GET, path, None, false).await;
        match retry_delay(&first) {
            Some(delay) => {
                tokio::time::sleep(delay).await;
                self.request(Method::GET, path, None, false).await
            }
            None => first,
        }
    }
}
fn retry_delay<T>(result: &Result<T, CliError>) -> Option<Duration> {
    match result {
        Err(CliError::Api {
            status: 429,
            retry_after_secs: Some(seconds),
            ..
        }) => {
            let delay = Duration::from_secs(*seconds);
            (delay <= MAX_RETRY_AFTER).then_some(delay)
        }
        Err(CliError::Api { status, .. }) if (500..=599).contains(status) => {
            Some(Duration::from_millis(100))
        }
        _ => None,
    }
}
fn api_error(
    status: StatusCode,
    text: &str,
    retry_after_secs: Option<u64>,
    outcome: ResponseOutcome,
) -> CliError {
    #[derive(Deserialize)]
    struct Body {
        code: String,
        message: String,
        details: Option<Value>,
    }
    match serde_json::from_str::<Body>(text) {
        Ok(error) => {
            let message = if status == StatusCode::MISDIRECTED_REQUEST
                && error.code == "primary_only"
            {
                "the configured API endpoint refused this write; reconnect a saved workspace to the primary API, or unset FOMKEE_API_URL to use the default hosted API with an environment token".into()
            } else {
                error.message
            };
            CliError::Api {
                status: status.as_u16(),
                code: error.code,
                message,
                details: error.details,
                retry_after_secs,
            }
        }
        Err(source) => CliError::Response(ResponseFailure {
            status: status.as_u16(),
            outcome,
            source: ResponseCause::Decode(source),
            retry_after_secs,
        }),
    }
}
#[async_trait]
impl FomkeeApi for HttpFomkeeApi {
    async fn workspace(&self, workspace: &WorkspaceId) -> Result<Response<Workspace>, CliError> {
        self.read_request(&format!("api/workspaces/{workspace}"))
            .await
    }
    async fn session(&self) -> Result<Response<Session>, CliError> {
        self.read_request("api/session").await
    }
    async fn entitlements(
        &self,
        workspace: &WorkspaceId,
    ) -> Result<Response<Entitlements>, CliError> {
        self.read_request(&format!("api/workspaces/{workspace}/entitlements"))
            .await
    }
    async fn list_monitors(
        &self,
        workspace: &WorkspaceId,
        limit: u32,
        after: Option<&str>,
    ) -> Result<Response<MonitorPage>, CliError> {
        let query = page_query(limit, after)?;
        self.read_request(&format!("api/workspaces/{workspace}/monitors?{query}"))
            .await
    }
    async fn get_monitor(
        &self,
        workspace: &WorkspaceId,
        monitor: &MonitorId,
    ) -> Result<Response<Monitor>, CliError> {
        self.read_request(&format!("api/workspaces/{workspace}/monitors/{monitor}"))
            .await
    }
    async fn dry_run(
        &self,
        workspace: &WorkspaceId,
        body: DryRunInput,
    ) -> Result<Response<DryRun>, CliError> {
        self.request(
            Method::POST,
            &format!("api/workspaces/{workspace}/monitors/dry-run"),
            Some(serialize_body(&body)?),
            false,
        )
        .await
    }
    async fn create_monitor(
        &self,
        workspace: &WorkspaceId,
        input: CreateInput,
    ) -> Result<Response<CreatedMonitor>, CliError> {
        self.request(
            Method::POST,
            &format!("api/workspaces/{workspace}/monitors"),
            Some(serialize_body(&input)?),
            true,
        )
        .await
    }
    async fn update_monitor(
        &self,
        workspace: &WorkspaceId,
        monitor: &MonitorId,
        input: UpdateInput,
    ) -> Result<Response<Monitor>, CliError> {
        self.request(
            Method::PUT,
            &format!("api/workspaces/{workspace}/monitors/{monitor}"),
            Some(serialize_body(&input)?),
            true,
        )
        .await
    }
    async fn lifecycle(
        &self,
        workspace: &WorkspaceId,
        monitor: &MonitorId,
        input: LifecycleInput,
    ) -> Result<Response<Monitor>, CliError> {
        let (operation, body) = match input {
            LifecycleInput::Pause(body) => ("pause", Some(serialize_body(&body)?)),
            LifecycleInput::Resume => ("resume", None),
            LifecycleInput::Disable(body) => ("disable", Some(serialize_body(&body)?)),
            LifecycleInput::Enable => ("enable", None),
        };
        self.request(
            Method::POST,
            &format!("api/workspaces/{workspace}/monitors/{monitor}/{operation}"),
            body,
            true,
        )
        .await
    }
    async fn delete_monitor(
        &self,
        workspace: &WorkspaceId,
        monitor: &MonitorId,
    ) -> Result<(), CliError> {
        self.request::<()>(
            Method::DELETE,
            &format!("api/workspaces/{workspace}/monitors/{monitor}"),
            None,
            true,
        )
        .await
        .map(|_| ())
    }
}

fn serialize_body(value: &impl Serialize) -> Result<Value, CliError> {
    serde_json::to_value(value)
        .map_err(|_| CliError::InvalidInput("cannot serialize API request".into()))
}

mod alerting;

fn page_query(limit: u32, after: Option<&str>) -> Result<String, CliError> {
    if !(1..=PAGE_SIZE).contains(&limit) {
        return Err(CliError::InvalidInput(format!(
            "--limit must be between 1 and {PAGE_SIZE}"
        )));
    }
    let mut query = form_urlencoded::Serializer::new(String::new());
    query.append_pair("limit", &limit.to_string());
    if let Some(after) = after {
        query.append_pair("after", after);
    }
    Ok(query.finish())
}
