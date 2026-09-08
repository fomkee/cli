#![cfg(test)]

use std::error::Error;
use std::io::{self, Write};
use std::net::TcpListener;
use std::thread::{self, JoinHandle};
mod raw_http;
use raw_http::{accept, read_request};

use fomkee_cli::client::{FomkeeApi, HttpFomkeeApi};
use fomkee_cli::config::{ApiToken, Config, api_url};
use fomkee_cli::error::CliError;
use fomkee_cli::monitoring::CreateInput;
use serde_json::{Value, json};

const WORKSPACE: &str = "00000000-0000-0000-0000-000000000001";

struct Server {
    client: HttpFomkeeApi,
    task: Option<JoinHandle<io::Result<()>>>,
}

impl Server {
    fn responding(status: u16, body: &'static str, length: usize) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let config = Config {
            base_url: api_url(&format!("http://{}/", listener.local_addr().unwrap())).unwrap(),
            token: ApiToken::new("fk_response_fixture".into()).unwrap(),
        };
        let task = thread::spawn(move || {
            let mut stream = accept(&listener)?;
            read_request(&mut stream)?;
            write!(
                stream,
                "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n{body}"
            )?;
            stream.flush()
        });
        Self {
            client: HttpFomkeeApi::new(&config).unwrap(),
            task: Some(task),
        }
    }

    async fn create(&self) -> CliError {
        let input = CreateInput::try_from(
            json!({"config_type":"http","url":"https://example.com","interval_secs":60}),
        )
        .unwrap();
        self.client
            .create_monitor(&WORKSPACE.parse().unwrap(), input)
            .await
            .unwrap_err()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let result = self.task.take().unwrap().join();
        match result {
            Ok(Ok(())) => {}
            failure if thread::panicking() => {
                eprintln!("fixture server failed during cleanup: {failure:?}")
            }
            failure => assert!(
                matches!(failure, Ok(Ok(()))),
                "fixture server failed: {failure:?}"
            ),
        }
    }
}

fn assert_acknowledged_failure(error: &CliError, exit: u8, category: &str) {
    let output = serde_json::to_value(error.as_json()).unwrap();
    assert_eq!(error.exit_code(), exit);
    assert_eq!(output.get("status"), Some(&json!(201)));
    assert_eq!(output.get("category"), Some(&json!(category)));
    assert_eq!(
        output.get("code"),
        Some(&json!("mutation_result_unavailable"))
    );
    assert!(error.to_string().contains("do not retry automatically"));
    assert!(error.source().is_some());
}

#[tokio::test]
async fn test_truncated_create_response_preserves_acknowledgement_and_transport_exit() {
    // Arrange
    let server = Server::responding(201, "{", 99);
    // Act
    let error = server.create().await;
    // Assert
    assert_acknowledged_failure(&error, 7, "transport");
}

#[tokio::test]
async fn test_invalid_create_json_preserves_acknowledgement_and_protocol_exit() {
    // Arrange
    let server = Server::responding(201, "{", 1);
    // Act
    let error = server.create().await;
    // Assert
    assert_acknowledged_failure(&error, 8, "protocol");
}

#[tokio::test]
async fn test_valid_json_with_invalid_creation_shape_is_not_success() {
    // Arrange
    let server = Server::responding(201, "{}", 2);
    // Act
    let error = server.create().await;
    // Assert
    assert_acknowledged_failure(&error, 8, "protocol");
}

#[tokio::test]
async fn test_truncated_rejection_preserves_status_without_claiming_success() {
    // Arrange
    let server = Server::responding(403, "{", 99);
    // Act
    let error = server.create().await;
    // Assert
    let output = serde_json::to_value(error.as_json()).unwrap();
    assert_eq!(output.get("status"), Some(&json!(403)));
    assert_eq!(
        output.get("code"),
        Some(&json!("mutation_error_unavailable"))
    );
    assert!(!error.to_string().contains("acknowledged"));
}

#[tokio::test]
async fn test_read_decode_error_has_no_mutation_recovery_code() {
    // Arrange
    let server = Server::responding(200, "{}", 2);
    // Act
    let error = server.client.session().await.unwrap_err();
    // Assert
    let output: Value = serde_json::to_value(error.as_json()).unwrap();
    assert!(output.get("code").is_none());
    assert!(!error.to_string().contains("mutation"));
}

#[tokio::test]
async fn test_invalid_mutation_error_json_preserves_status_source_and_recovery() {
    // Arrange
    let server = Server::responding(403, "{}", 2);
    // Act
    let error = server.create().await;
    // Assert
    assert_unreadable_api_error(&error, Some("mutation_error_unavailable"));
}

#[tokio::test]
async fn test_invalid_read_error_json_preserves_status_and_source_without_mutation_hint() {
    // Arrange
    let server = Server::responding(403, "{}", 2);
    // Act
    let error = server.client.session().await.unwrap_err();
    // Assert
    assert_unreadable_api_error(&error, None);
}

fn assert_unreadable_api_error(error: &CliError, code: Option<&str>) {
    let output = serde_json::to_value(error.as_json()).unwrap();
    assert_eq!(error.exit_code(), 8);
    assert_eq!(output.get("status"), Some(&json!(403)));
    assert_eq!(output.get("category"), Some(&json!("protocol")));
    assert_eq!(output.get("code").and_then(Value::as_str), code);
    assert!(!error.to_string().contains("acknowledged"));
    assert!(error.source().unwrap().source().is_some());
}
