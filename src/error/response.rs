use std::fmt::{self, Debug, Display, Formatter};

use thiserror::Error;

/// What is known from a response before decoding its result failed.
#[derive(Debug, Clone, Copy)]
pub enum ResponseOutcome {
    Read,
    MutationAcknowledged,
    MutationErrorReported,
}

/// The decoding phase determines the existing transport/protocol exit code.
#[derive(Error)]
pub enum ResponseCause {
    #[error("response body could not be read")]
    Body(#[source] reqwest::Error),
    #[error("response did not match the public JSON shape")]
    Decode(#[source] serde_json::Error),
}

impl Debug for ResponseCause {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

/// A response failure retaining known status and safe mutation recovery context.
#[derive(Debug, Error)]
#[error("HTTP {status}: {source}; {outcome}")]
pub struct ResponseFailure {
    pub status: u16,
    pub outcome: ResponseOutcome,
    #[source]
    pub source: ResponseCause,
    pub retry_after_secs: Option<u64>,
}

impl Display for ResponseOutcome {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Read => "the requested result is unavailable",
            Self::MutationAcknowledged => "the API acknowledged the mutation but its result is unavailable; do not retry automatically; inspect current state first",
            Self::MutationErrorReported => "the API reported an error but its details are unavailable; inspect current state before retrying",
        })
    }
}

impl ResponseFailure {
    pub(crate) fn category(&self) -> &'static str {
        match self.source {
            ResponseCause::Body(_) => "transport",
            ResponseCause::Decode(_) => "protocol",
        }
    }
    pub(crate) fn exit_code(&self) -> u8 {
        match self.source {
            ResponseCause::Body(_) => 7,
            ResponseCause::Decode(_) => 8,
        }
    }
    pub(crate) fn code(&self) -> Option<String> {
        match self.outcome {
            ResponseOutcome::Read => None,
            ResponseOutcome::MutationAcknowledged => Some("mutation_result_unavailable".into()),
            ResponseOutcome::MutationErrorReported => Some("mutation_error_unavailable".into()),
        }
    }
}
