#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo)]

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    fomkee_cli::run_process().await
}
