use crate::config::{ApiToken, Config};
use serde_json::{Value, json};
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use url::Url;

use axum::Router;
use axum::extract::{OriginalUri, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use super::*;

async fn serve(app: Router) -> Result<(Url, JoinHandle<std::io::Result<()>>), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let task = tokio::spawn(async move { axum::serve(listener, app).await });
    Ok((Url::parse(&format!("http://{address}/"))?, task))
}

fn client(base_url: Url) -> HttpFomkeeApi {
    HttpFomkeeApi::new(&Config {
        base_url,
        token: ApiToken::new("fk_test_fixture".into()).unwrap(),
    })
    .unwrap()
}

#[tokio::test]
async fn test_retries_rate_limited_read_after_retry_after() {
    let result = rate_limited_read_scenario().await;
    assert!(result.is_ok(), "scenario failed: {result:?}");
}

async fn rate_limited_read_scenario() -> Result<(), Box<dyn Error>> {
    // Arrange
    let first_request = Arc::new(AtomicBool::new(true));
    let app = Router::new()
        .route(
            "/api/session",
            get(|State(first): State<Arc<AtomicBool>>| async move {
                if first.swap(false, Ordering::SeqCst) {
                    let mut headers = HeaderMap::new();
                    headers.insert("retry-after", HeaderValue::from_static("0"));
                    return (
                        StatusCode::TOO_MANY_REQUESTS,
                        headers,
                        r#"{"code":"rate_limited","message":"slow down"}"#,
                    )
                        .into_response();
                }
                (
                    StatusCode::OK,
                    r#"{"workspace_id":"00000000-0000-0000-0000-000000000001"}"#,
                )
                    .into_response()
            }),
        )
        .with_state(first_request.clone());
    let (base_url, task) = serve(app).await?;

    // Act
    let response = client(base_url).session().await?;

    // Assert
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let response = serde_json::to_value(response)?;
    assert_eq!(
        response.get("workspace_id"),
        Some(&json!("00000000-0000-0000-0000-000000000001"))
    );
    assert!(!first_request.load(Ordering::SeqCst));
    Ok(())
}

#[tokio::test]
async fn test_preserves_api_status_code_details_and_retry_metadata() {
    let result = api_error_scenario().await;
    assert!(result.is_ok(), "scenario failed: {result:?}");
}

async fn api_error_scenario() -> Result<(), Box<dyn Error>> {
    // Arrange
    let app = Router::new().route(
        "/api/session",
        get(|| async {
            let mut headers = HeaderMap::new();
            headers.insert("retry-after", HeaderValue::from_static("30"));
            (
                StatusCode::TOO_MANY_REQUESTS,
                headers,
                r#"{"code":"rate_limited","message":"slow down","details":{"scope":"workspace_per_node","window":"fixed_utc_minute"}}"#,
            )
        }),
    );
    let (base_url, task) = serve(app).await?;

    // Act
    let response = client(base_url).session().await;

    // Assert
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    match response {
        Err(CliError::Api {
            status,
            code,
            retry_after_secs,
            details,
            ..
        }) => {
            assert_eq!(status, 429);
            assert_eq!(code, "rate_limited");
            assert_eq!(retry_after_secs, Some(30));
            assert_eq!(
                details.and_then(|value| value.get("scope").cloned()),
                Some(json!("workspace_per_node"))
            );
        }
        other => assert!(
            matches!(other, Err(CliError::Api { .. })),
            "unexpected response: {other:?}"
        ),
    }
    Ok(())
}

#[tokio::test]
async fn test_returns_one_monitor_page_with_an_opaque_next_cursor() {
    let result = pagination_scenario().await;
    assert!(result.is_ok(), "scenario failed: {result:?}");
}

#[tokio::test]
async fn test_rejects_monitor_page_larger_than_server_maximum() -> Result<(), Box<dyn Error>> {
    // Arrange
    let base_url = Url::parse("http://127.0.0.1:1/")?;
    let workspace = "00000000-0000-0000-0000-000000000001".parse()?;

    // Act
    let response = client(base_url).list_monitors(&workspace, 101, None).await;

    // Assert
    assert!(matches!(response, Err(CliError::InvalidInput(_))));
    Ok(())
}

async fn pagination_scenario() -> Result<(), Box<dyn Error>> {
    // Arrange
    let app = Router::new().route(
        "/api/workspaces/00000000-0000-0000-0000-000000000001/monitors",
        get(|OriginalUri(uri): OriginalUri| async move {
            axum::Json(json!({
                "items":[{"id":"00000000-0000-0000-0000-000000000002","workspace_id":"00000000-0000-0000-0000-000000000001","name":"Example","state":"active","config_type":"http","method":"GET","url":"https://example.com","interval_secs":60,"query":uri.query()}],
                "next_cursor":"next cursor/+?"
            }))
        }),
    );
    let (base_url, task) = serve(app).await?;

    // Act
    let workspace = "00000000-0000-0000-0000-000000000001".parse()?;
    let response = client(base_url)
        .list_monitors(&workspace, 2, Some("previous cursor/+?"))
        .await?;

    // Assert
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let response = serde_json::to_value(response)?;
    assert_eq!(
        response
            .get("items")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(response.get("next_cursor"), Some(&json!("next cursor/+?")));
    let query = response
        .get("items")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("query"));
    assert_eq!(
        query,
        Some(&json!("limit=2&after=previous+cursor%2F%2B%3F"))
    );
    Ok(())
}

#[tokio::test]
async fn test_rejects_malformed_success_response() {
    let result = malformed_response_scenario().await;
    assert!(result.is_ok(), "scenario failed: {result:?}");
}

async fn malformed_response_scenario() -> Result<(), Box<dyn Error>> {
    // Arrange
    let app = Router::new().route(
        "/api/session",
        get(|| async { (StatusCode::OK, "not-json") }),
    );
    let (base_url, task) = serve(app).await?;

    // Act
    let response = client(base_url).session().await;

    // Assert
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(matches!(response, Err(CliError::Response(_))));
    Ok(())
}

#[tokio::test]
async fn test_does_not_retry_monitor_creation_after_server_error() {
    let result = mutation_no_retry_scenario().await;
    assert!(result.is_ok(), "scenario failed: {result:?}");
}

#[tokio::test]
async fn test_preserves_primary_only_worker_rejection() {
    let result = primary_only_scenario().await;
    assert!(result.is_ok(), "scenario failed: {result:?}");
}

async fn primary_only_scenario() -> Result<(), Box<dyn Error>> {
    // Arrange
    let app = Router::new().route(
        "/api/workspaces/00000000-0000-0000-0000-000000000001/monitors",
        axum::routing::post(|| async {
            (
                StatusCode::MISDIRECTED_REQUEST,
                r#"{"code":"primary_only","message":"use primary"}"#,
            )
        }),
    );
    let (base_url, task) = serve(app).await?;
    let input = CreateInput::try_from(
        json!({"config_type":"heartbeat","schedule_type":"interval","period_secs":60}),
    )?;

    // Act
    let workspace = "00000000-0000-0000-0000-000000000001".parse()?;
    let response = client(base_url).create_monitor(&workspace, input).await;

    // Assert
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(matches!(
        response,
        Err(CliError::Api {
            status: 421,
            ref code,
            ref message,
            ..
        }) if code == "primary_only" && message.contains("unset FOMKEE_API_URL")
    ));
    Ok(())
}

async fn mutation_no_retry_scenario() -> Result<(), Box<dyn Error>> {
    // Arrange
    let first_request = Arc::new(AtomicBool::new(true));
    let app = Router::new()
        .route(
            "/api/workspaces/00000000-0000-0000-0000-000000000001/monitors",
            axum::routing::post(|State(first): State<Arc<AtomicBool>>| async move {
                if first.swap(false, Ordering::SeqCst) {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        r#"{"code":"internal_error","message":"failed"}"#,
                    );
                }
                (StatusCode::CREATED, r#"{"monitor":{"id":"unexpected"}}"#)
            }),
        )
        .with_state(first_request);
    let (base_url, task) = serve(app).await?;
    let input = CreateInput::try_from(
        json!({"config_type":"heartbeat","schedule_type":"interval","period_secs":60}),
    )?;

    // Act
    let workspace = "00000000-0000-0000-0000-000000000001".parse()?;
    let response = client(base_url).create_monitor(&workspace, input).await;

    // Assert
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(matches!(response, Err(CliError::Api { status: 500, .. })));
    Ok(())
}

#[tokio::test]
async fn test_preserves_one_time_heartbeat_secret_in_requested_create_result() {
    let result = heartbeat_secret_scenario().await;
    assert!(result.is_ok(), "scenario failed: {result:?}");
}

async fn heartbeat_secret_scenario() -> Result<(), Box<dyn Error>> {
    // Arrange
    let app = Router::new().route(
        "/api/workspaces/00000000-0000-0000-0000-000000000001/monitors",
        axum::routing::post(|| async {
            (
                StatusCode::CREATED,
                axum::Json(json!({"monitor":{"id":"00000000-0000-0000-0000-000000000002","workspace_id":"00000000-0000-0000-0000-000000000001","name":"Heartbeat","state":"active","config_type":"heartbeat","schedule_type":"interval","period_secs":60},"heartbeat_secret":"one-time-secret"})),
            )
        }),
    );
    let (base_url, task) = serve(app).await?;
    let input = CreateInput::try_from(
        json!({"config_type":"heartbeat","schedule_type":"interval","period_secs":60}),
    )?;

    // Act
    let workspace = "00000000-0000-0000-0000-000000000001".parse()?;
    let response = client(base_url).create_monitor(&workspace, input).await?;

    // Assert
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let response = serde_json::to_value(response)?;
    assert_eq!(
        response.get("heartbeat_secret"),
        Some(&json!("one-time-secret"))
    );
    assert_eq!(
        response
            .get("monitor")
            .and_then(|monitor| monitor.get("id")),
        Some(&json!("00000000-0000-0000-0000-000000000002"))
    );
    Ok(())
}
