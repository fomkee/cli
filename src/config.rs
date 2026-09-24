use std::env;

use url::Url;

use crate::error::CliError;

/// Hosted primary origin used unless the invocation selects another API.
pub const DEFAULT_API_URL: &str = "https://primary.fomkee.com";

pub(crate) const DEFAULT_APP_URL: &str = "https://app.fomkee.com";
const LEGACY_API_URL: &str = "https://primary.fomkee.dev";

/// Process configuration whose secret token deliberately has no Debug implementation.
pub struct Config {
    pub base_url: Url,
    pub token: ApiToken,
}

/// Workspace credential whose diagnostic representation is always redacted.
pub struct ApiToken(String);
impl std::fmt::Debug for ApiToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ApiToken([REDACTED])")
    }
}
impl ApiToken {
    /// Parse and trim a workspace token without accepting a Bearer prefix.
    pub fn new(value: String) -> Result<Self, CliError> {
        let value = value.trim();
        if value.is_empty() {
            return Err(CliError::MissingCredentials);
        }
        if !value.starts_with("fk_") || !value.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(CliError::InvalidInput(
                "provide a workspace API token beginning with fk_, without the Bearer prefix"
                    .into(),
            ));
        }
        Ok(Self(value.to_owned()))
    }
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl Config {
    /// Resolve an ephemeral origin and token from the process environment.
    pub fn from_environment() -> Result<Self, CliError> {
        let raw_url = env::var("FOMKEE_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.into());
        let base_url = api_url(&raw_url)?;
        let token = env::var("FOMKEE_API_TOKEN").map_err(|_| CliError::MissingCredentials)?;
        Ok(Self {
            base_url,
            token: ApiToken::new(token)?,
        })
    }
}

/// Validate an HTTPS origin, allowing HTTP only for loopback development.
pub fn api_url(value: &str) -> Result<Url, CliError> {
    let url = Url::parse(value).map_err(|_| invalid_origin())?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if !(url.scheme() == "https" || url.scheme() == "http" && local)
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
    {
        return Err(invalid_origin());
    }
    Ok(url)
}

pub(crate) fn is_hosted_api_url(url: &Url) -> bool {
    matches!(
        url.as_str().trim_end_matches('/'),
        DEFAULT_API_URL | LEGACY_API_URL
    )
}

fn invalid_origin() -> CliError {
    CliError::InvalidInput("API URL must be an HTTPS origin without credentials, path, query, or fragment (HTTP is allowed for loopback development)".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_and_legacy_hosted_origins_are_recognized() {
        // Arrange
        let current = api_url(DEFAULT_API_URL).unwrap();
        let legacy = api_url(LEGACY_API_URL).unwrap();

        // Act
        let recognized = (is_hosted_api_url(&current), is_hosted_api_url(&legacy));

        // Assert
        assert_eq!(recognized, (true, true));
    }

    #[test]
    fn test_custom_origin_is_not_recognized_as_hosted() {
        // Arrange
        let custom = api_url("https://monitoring.example.com").unwrap();

        // Act
        let hosted = is_hosted_api_url(&custom);

        // Assert
        assert!(!hosted);
    }
}
