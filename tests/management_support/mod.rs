use axum::body::{Body, to_bytes};
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::io::Write;
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

pub const WS: &str = "00000000-0000-0000-0000-000000000001";
pub const MONITOR: &str = "00000000-0000-0000-0000-000000000002";
pub const TARGET: &str = "00000000-0000-0000-0000-000000000003";
pub const BINDING: &str = "00000000-0000-0000-0000-000000000004";

pub struct Step {
    method: &'static str,
    path: String,
    input: Value,
    status: StatusCode,
    output: Value,
}
impl Step {
    pub fn new(method: &'static str, path: &str, input: Value, output: Value) -> Self {
        Self {
            method,
            path: format!("/api/workspaces/{WS}/{path}"),
            input,
            status: StatusCode::OK,
            output,
        }
    }
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }
}
async fn handle(State(steps): State<Arc<Mutex<VecDeque<Step>>>>, request: Request) -> Response {
    assert_eq!(
        request.headers().get("authorization").unwrap(),
        "Bearer fk_fixture"
    );
    let (parts, body) = request.into_parts();
    let bytes = to_bytes(body, 1024 * 1024).await.unwrap();
    let input = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    let step = steps.lock().await.pop_front().unwrap();
    assert_eq!(
        (parts.method.as_str(), parts.uri.to_string(), input),
        (step.method, step.path, step.input)
    );
    if step.status == StatusCode::NO_CONTENT {
        return (StatusCode::NO_CONTENT, Body::empty()).into_response();
    }
    (step.status, Json(step.output)).into_response()
}
pub async fn run(steps: Vec<Step>, args: &[&str], input: &str) -> Output {
    let mut queue = VecDeque::from(steps);
    queue.push_front(Step {
        method: "GET",
        path: "/api/session".into(),
        input: Value::Null,
        status: StatusCode::OK,
        output: json!({"workspace_id": WS}),
    });
    let state = Arc::new(Mutex::new(queue));
    let app = Router::new().fallback(handle).with_state(state.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}/", listener.local_addr().unwrap());
    let task = tokio::spawn(async move { axum::serve(listener, app).await });
    let directory = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_fomkeecli"))
        .env("FOMKEE_API_TOKEN", "fk_fixture")
        .env("FOMKEE_API_URL", origin)
        .env("FOMKEE_CONFIG_DIR", directory.path())
        .env("FOMKEE_NO_UPDATE_CHECK", "1")
        .env_remove("FOMKEE_PROFILE")
        .env_remove("FOMKEE_WORKSPACE_ID")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(
        state.lock().await.is_empty(),
        "expected API requests missing: {output:?}"
    );
    output
}
pub fn succeeded(output: Output) -> Value {
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}
pub fn destination() -> Value {
    json!({"id": TARGET, "workspace_id": WS, "name": "Chat", "channel_type": "telegram", "state": {"state": "available"},
        "config": {"telegram_chat_id": "-100123", "telegram_bot_token_configured": true}, "created_at": "2026-09-15T00:00:00Z", "updated_at": "2026-09-15T00:00:00Z", "future": "preserved"})
}
pub fn assignment() -> Value {
    json!({"id": BINDING, "workspace_id": WS, "monitor_id": MONITOR, "alert_target_id": TARGET, "created_at": "2026-09-15T00:00:00Z"})
}
pub fn monitor() -> Value {
    json!({"id": MONITOR, "workspace_id": WS, "state": "active", "name": "Before", "description": null, "tags": [], "sla_target_parts_per_million": null,
        "config_type": "http", "url": "https://example.com", "method": "GET", "interval_secs": 60, "timeout_secs": 10, "retry_max_attempts": 3, "retry_base_backoff_ms": 1000, "retry_max_backoff_ms": 30000,
        "headers": [], "auth": {"type": "bearer"}, "body": null, "expected_status": "any_success", "response_time_max_ms": null})
}
pub fn replacement() -> Value {
    let mut input = monitor();
    let object = input.as_object_mut().unwrap();
    object.remove("id");
    object.remove("workspace_id");
    object.remove("state");
    object.insert("auth".into(), json!({"action": "preserve"}));
    object.insert("name".into(), json!("After"));
    input
}
