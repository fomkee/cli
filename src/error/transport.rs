use std::fmt::{self, Debug, Display, Formatter};
use std::io;

use thiserror::Error;

/// Typed transport cause with safe outer diagnostics; the original cause is retained.
#[derive(Error)]
pub enum TransportFailure {
    #[error("cannot write JSON output")]
    JsonOutput(#[source] serde_json::Error),
    #[error("HTTP request could not be completed")]
    Http(#[source] reqwest::Error),
    #[error("{operation}")]
    Io {
        operation: &'static str,
        #[source]
        source: io::Error,
    },
    #[error("{0}")]
    Message(String),
}

impl From<String> for TransportFailure {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for TransportFailure {
    fn from(value: &str) -> Self {
        Self::Message(value.into())
    }
}

impl Debug for TransportFailure {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}
