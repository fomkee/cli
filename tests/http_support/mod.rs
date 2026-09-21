use std::io::{self, Write};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing::get};
use fomkee_cli::config::{ApiToken, Config, api_url};
use fomkee_cli::workspace::model::{
    CredentialBackend, CredentialId, WorkspaceAlias, WorkspaceProfile, WorkspaceRegistry,
};
use fomkee_cli::workspace::store::{FileWorkspaceStore, WorkspaceStore};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub const TOKEN: &str = "fk_http_fixture_not_a_real_key";
pub const WORKSPACE: &str = "00000000-0000-0000-0000-000000000003";

pub struct ApiServer {
    pub origin: String,
    calls: Arc<AtomicUsize>,
    task: JoinHandle<io::Result<()>>,
    creations: Arc<Mutex<Vec<Value>>>,
}

impl ApiServer {
    pub async fn start() -> Self {
        Self::with_minimum(json!(60)).await
    }

    pub async fn with_minimum(minimum: Value) -> Self {
        let calls = Arc::new(AtomicUsize::new(0));
        let creations = Arc::new(Mutex::new(Vec::new()));
        let captured = creations.clone();
        let app = Router::new()
            .route("/api/session", get(session))
            .route(
                &format!("/api/workspaces/{WORKSPACE}"),
                get(|| async { Json(json!({"id": WORKSPACE, "name": "Personal"})) }),
            )
            .route(
                &format!("/api/workspaces/{WORKSPACE}/monitors"),
                get(|| async { Json(json!({"items": [], "next_cursor": null})) }).post(
                    move |Json(input): Json<Value>| {
                        let captured = captured.clone();
                        async move {
                            captured.lock().await.push(input.clone());
                            created_response(input)
                        }
                    },
                ),
            )
            .route(
                &format!("/api/workspaces/{WORKSPACE}/entitlements"),
                get(move || {
                    let minimum = minimum.clone();
                    async move {
                        Json(
                            json!({"plan": {"revision": {"entitlements": {"monitoring": {
                                "minimum_check_interval_seconds": minimum
                            }}}}}),
                        )
                    }
                }),
            )
            .layer(from_fn_with_state(calls.clone(), count_requests))
            .with_state(calls.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}/", listener.local_addr().unwrap());
        let task = tokio::spawn(async move { axum::serve(listener, app).await });
        Self {
            origin,
            calls,
            task,
            creations,
        }
    }

    pub async fn assert_created_input(&self, expected: Value) {
        assert_eq!(*self.creations.lock().await, vec![expected]);
    }

    pub fn config(&self) -> Config {
        Config {
            base_url: api_url(&self.origin).unwrap(),
            token: ApiToken::new(TOKEN.into()).unwrap(),
        }
    }

    pub async fn finish(self, requests: usize) {
        self.task.abort();
        let error = self.task.await.unwrap_err();
        assert!(error.is_cancelled(), "server task failed: {error}");
        assert_eq!(self.calls.load(Ordering::SeqCst), requests);
    }
}

fn created_response(mut input: Value) -> Json<Value> {
    let object = input.as_object_mut().unwrap();
    object.insert("id".into(), json!("00000000-0000-0000-0000-000000000004"));
    object.insert("workspace_id".into(), json!(WORKSPACE));
    object.insert("state".into(), json!("active"));
    if object.get("config_type").and_then(Value::as_str) != Some("heartbeat") {
        object.entry("method").or_insert(json!("GET"));
    }
    Json(json!({"monitor": input}))
}

async fn count_requests(
    State(calls): State<Arc<AtomicUsize>>,
    request: Request,
    next: Next,
) -> Response {
    calls.fetch_add(1, Ordering::SeqCst);
    next.run(request).await
}

async fn session(headers: HeaderMap) -> Response {
    if headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        != Some(&format!("Bearer {TOKEN}"))
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"code": "invalid_api_key", "message": "invalid API key"})),
        )
            .into_response();
    }
    Json(json!({"workspace_id": WORKSPACE, "role": "ApiKey"})).into_response()
}

pub struct Process {
    pub directory: TempDir,
}

impl Process {
    pub fn new() -> Self {
        let directory = tempdir().unwrap();
        #[cfg(unix)]
        {
            use std::fs::{self, Permissions};
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(directory.path(), Permissions::from_mode(0o700)).unwrap();
        }
        Self { directory }
    }

    pub fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_fomkeecli"));
        command
            .env("FOMKEE_CONFIG_DIR", self.directory.path())
            .env_remove("FOMKEE_API_URL")
            .env_remove("FOMKEE_API_TOKEN")
            .env_remove("FOMKEE_PROFILE")
            .env_remove("FOMKEE_WORKSPACE_ID");
        command
    }

    pub fn authorized(&self, server: &ApiServer) -> Command {
        let mut command = self.json_command();
        command
            .env("FOMKEE_API_TOKEN", TOKEN)
            .env("FOMKEE_API_URL", &server.origin);
        command
    }

    pub fn json_command(&self) -> Command {
        let mut command = self.command();
        command.arg("--json");
        command
    }

    pub fn save_profile(&self, alias: &str, origin: &str) {
        let store = FileWorkspaceStore::open(self.directory.path().to_owned()).unwrap();
        let mut registry = WorkspaceRegistry::default();
        let alias: WorkspaceAlias = alias.parse().unwrap();
        registry.active = Some(alias.clone());
        registry.workspaces.insert(
            alias,
            WorkspaceProfile {
                api_url: origin.into(),
                workspace_id: WORKSPACE.parse().unwrap(),
                name: "Personal".into(),
                credential_id: CredentialId::generate().unwrap(),
                credential_store: CredentialBackend::Keyring,
            },
        );
        store.save(&registry).unwrap();
    }

    pub fn connect_stdin(&self, server: &ApiServer, token: &str) -> Output {
        let mut process = self
            .command()
            .args([
                "workspace",
                "connect",
                "personal",
                "--token-stdin",
                "--credential-store",
                "file",
                "--api-url",
                &server.origin,
                "--output",
                "json",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        process
            .stdin
            .take()
            .unwrap()
            .write_all(token.as_bytes())
            .unwrap();
        let output = process.wait_with_output().unwrap();
        assert!(!String::from_utf8_lossy(&output.stdout).contains(token));
        assert!(!String::from_utf8_lossy(&output.stderr).contains(token));
        output
    }
}

pub fn assert_success(output: &Output, expected: Value) {
    assert!(output.status.success(), "{:?}", output);
    assert!(output.stderr.is_empty(), "{:?}", output);
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        expected
    );
}

pub fn assert_failure(output: &Output, exit: i32, category: &str) {
    assert_eq!(output.status.code(), Some(exit), "{output:?}");
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        error.get("category").and_then(Value::as_str),
        Some(category)
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains(TOKEN));
}
