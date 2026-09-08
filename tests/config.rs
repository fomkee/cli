#![cfg(test)]

use fomkee_cli::config::{ApiToken, api_url};
use fomkee_cli::error::CliError;

#[test]
fn test_plain_http_is_rejected_for_remote_credentials() {
    assert!(api_url("http://primary.fomkee.dev").is_err());
}

#[test]
fn test_loopback_http_is_available_for_development() {
    assert!(api_url("http://127.0.0.1:9100").is_ok());
}

#[test]
fn test_ipv6_loopback_http_is_available_for_development() {
    assert!(api_url("http://[::1]:9100").is_ok());
}

#[test]
fn test_url_credentials_are_rejected_without_echoing_them() {
    // Act
    let error = api_url("https://user:private-fixture@primary.fomkee.dev").unwrap_err();

    // Assert
    assert!(!error.to_string().contains("private-fixture"));
}

#[test]
fn test_api_origin_cannot_contain_a_path() {
    assert!(api_url("https://primary.fomkee.dev/api").is_err());
}

#[test]
fn test_api_origin_cannot_contain_a_query() {
    assert!(api_url("https://primary.fomkee.dev?token=private-fixture").is_err());
}

#[test]
fn test_api_origin_cannot_contain_a_fragment() {
    assert!(api_url("https://primary.fomkee.dev#private-fixture").is_err());
}

#[test]
fn test_bearer_prefix_is_rejected_without_echoing_token() {
    // Act
    let error = ApiToken::new("Bearer fk_private-fixture".into()).unwrap_err();

    // Assert
    assert!(!error.to_string().contains("fk_private-fixture"));
}

#[test]
fn test_empty_token_is_reported_as_missing_credentials() {
    assert!(matches!(
        ApiToken::new("\r\n ".into()),
        Err(CliError::MissingCredentials)
    ));
}

#[test]
fn test_newline_inside_token_is_rejected() {
    assert!(ApiToken::new("fk_private\nfixture".into()).is_err());
}
