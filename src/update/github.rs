use super::{Asset, Release, ReleaseSource};
use crate::error::CliError;
use crate::error::transport::TransportFailure;
use async_trait::async_trait;
use reqwest::{Client, redirect::Policy};
use std::time::Duration;

/// Anonymous HTTPS client dedicated to GitHub; never receives Fomkee credentials.
pub struct GitHubReleases {
    http: Client,
}
impl GitHubReleases {
    /// Build a bounded client for the canonical public release repository.
    pub fn new() -> Result<Self, CliError> {
        let http = Client::builder()
            .https_only(true)
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(60))
            .user_agent(concat!("fomkeecli/", env!("CARGO_PKG_VERSION")))
            .redirect(Policy::limited(5))
            .build()
            .map_err(http_error)?;
        Ok(Self { http })
    }
    async fn fetch(&self, url: &str, limit: usize) -> Result<Vec<u8>, CliError> {
        let mut response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(http_error)?
            .error_for_status()
            .map_err(http_error)?;
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(http_error)? {
            if body
                .len()
                .checked_add(chunk.len())
                .is_none_or(|size| size > limit)
            {
                return Err(CliError::InvalidInput(
                    "GitHub response exceeds the download limit".into(),
                ));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }
}
fn http_error(error: reqwest::Error) -> CliError {
    CliError::Transport(TransportFailure::Http(error))
}
#[async_trait]
impl ReleaseSource for GitHubReleases {
    async fn latest(&self) -> Result<Release, CliError> {
        let body = self
            .fetch(
                "https://api.github.com/repos/fomkee/cli/releases/latest",
                1024 * 1024,
            )
            .await?;
        serde_json::from_slice(&body)
            .map_err(|_| CliError::MalformedResponse("GitHub release metadata is invalid".into()))
    }
    async fn download(&self, asset: &Asset, limit: usize) -> Result<Vec<u8>, CliError> {
        if asset.size
            > u64::try_from(limit)
                .map_err(|_| CliError::InvalidInput("download limit is invalid".into()))?
        {
            return Err(CliError::InvalidInput(
                "release asset exceeds the download limit".into(),
            ));
        }
        let prefix = "https://github.com/fomkee/cli/releases/download/";
        if !asset.browser_download_url.starts_with(prefix) {
            return Err(CliError::InvalidInput(
                "release asset is outside the canonical GitHub repository".into(),
            ));
        }
        self.fetch(&asset.browser_download_url, limit).await
    }
}
