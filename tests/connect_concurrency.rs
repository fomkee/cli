#![cfg(test)]

mod raw_http;

use std::io::{self, Write};
use std::net::TcpListener;
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use fomkee_cli::workspace::model::{
    CredentialBackend, CredentialId, WorkspaceProfile, WorkspaceRegistry,
};
use fomkee_cli::workspace::store::{FileWorkspaceStore, WorkspaceStore, read_registry};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};

const WORKSPACE: &str = "00000000-0000-0000-0000-000000000001";

struct Server {
    origin: String,
    entered: Receiver<()>,
    release: Sender<()>,
    task: Option<JoinHandle<io::Result<()>>>,
}

impl Server {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let (entered_tx, entered) = mpsc::channel();
        let (release, release_rx) = mpsc::channel();
        let task = thread::spawn(move || serve(listener, entered_tx, release_rx));
        Self {
            origin,
            entered,
            release,
            task: Some(task),
        }
    }

    fn wait_for_identity_lookup(&self) {
        self.entered.recv_timeout(Duration::from_secs(5)).unwrap();
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        // A completed fixture may already have dropped the receiver.
        let _ = self.release.send(());
        let result = self.task.take().unwrap().join();
        match result {
            Ok(Ok(())) => {}
            failure if thread::panicking() => eprintln!("server cleanup: {failure:?}"),
            failure => assert!(matches!(failure, Ok(Ok(()))), "server cleanup: {failure:?}"),
        }
    }
}

fn serve(listener: TcpListener, entered: Sender<()>, release: Receiver<()>) -> io::Result<()> {
    let session = json!({"workspace_id":WORKSPACE,"role":"ApiKey"});
    let workspace = json!({"id":WORKSPACE,"name":"Remote"});
    for (index, body) in [session, workspace].into_iter().enumerate() {
        let mut stream = raw_http::accept(&listener)?;
        raw_http::read_request(&mut stream)?;
        if index == 1 {
            entered.send(()).map_err(io::Error::other)?;
            release
                .recv_timeout(Duration::from_secs(5))
                .map_err(io::Error::other)?;
        }
        let body = body.to_string();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )?;
        stream.flush()?;
    }
    Ok(())
}

struct Connecting {
    child: Option<Child>,
}

impl Connecting {
    fn finish(mut self) -> Output {
        self.child.take().unwrap().wait_with_output().unwrap()
    }
}

impl Drop for Connecting {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let result = child.wait();
            if let Err(error) = result {
                eprintln!("child cleanup: {error}");
            }
        }
    }
}

struct Scenario {
    directory: TempDir,
    server: Server,
}

impl Scenario {
    fn new() -> Self {
        let directory = tempdir().unwrap();
        #[cfg(unix)]
        {
            use std::fs::{self, Permissions};
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(directory.path(), Permissions::from_mode(0o700)).unwrap();
        }
        let store = FileWorkspaceStore::open(directory.path().to_owned()).unwrap();
        let mut registry = WorkspaceRegistry::default();
        registry
            .workspaces
            .insert("existing".parse().unwrap(), profile());
        store.save(&registry).unwrap();
        Self {
            directory,
            server: Server::start(),
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_fomkeecli"));
        command
            .env("FOMKEE_CONFIG_DIR", self.directory.path())
            .env_remove("FOMKEE_API_TOKEN")
            .env_remove("FOMKEE_API_URL")
            .env_remove("FOMKEE_PROFILE")
            .env_remove("FOMKEE_WORKSPACE_ID")
            .arg("--json");
        command
    }

    fn connect(&self, alias: &str) -> Connecting {
        let mut child = self
            .command()
            .args([
                "workspace",
                "connect",
                alias,
                "--api-url",
                &self.server.origin,
                "--credential-store",
                "file",
                "--token-stdin",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"fk_concurrent_fixture\n")
            .unwrap();
        Connecting { child: Some(child) }
    }

    fn duplicate_alias(&self) -> Value {
        let store = FileWorkspaceStore::open(self.directory.path().to_owned()).unwrap();
        let mut registry = store.load().unwrap();
        registry
            .workspaces
            .insert("race".parse().unwrap(), profile());
        store.save(&registry).unwrap();
        serde_json::to_value(registry).unwrap()
    }
}

fn profile() -> WorkspaceProfile {
    WorkspaceProfile {
        api_url: "https://primary.fomkee.dev".into(),
        workspace_id: WORKSPACE.parse().unwrap(),
        name: "Existing".into(),
        credential_id: CredentialId::generate().unwrap(),
        credential_store: CredentialBackend::Keyring,
    }
}

fn assert_concurrent_selection(
    scenario: &Scenario,
    listing: Output,
    selection: Output,
    connected: Output,
) {
    assert!(listing.status.success(), "{listing:?}");
    assert!(selection.status.success(), "{selection:?}");
    assert!(connected.status.success(), "{connected:?}");
    let registry = read_registry(scenario.directory.path()).unwrap();
    assert_eq!(registry.active.unwrap().to_string(), "existing");
    assert_eq!(registry.workspaces.len(), 2);
}

#[test]
fn test_network_validation_does_not_lock_out_listing_or_overwrite_concurrent_selection() {
    // Arrange
    let scenario = Scenario::new();
    let connecting = scenario.connect("new");
    scenario.server.wait_for_identity_lookup();
    // Act
    let listing = scenario
        .command()
        .args(["workspace", "list"])
        .output()
        .unwrap();
    let selection = scenario
        .command()
        .args(["workspace", "use", "existing"])
        .output()
        .unwrap();
    scenario.server.release.send(()).unwrap();
    let connected = connecting.finish();
    // Assert
    assert_concurrent_selection(&scenario, listing, selection, connected);
}

#[test]
fn test_alias_added_during_validation_cannot_be_overwritten_or_save_an_orphan_token() {
    // Arrange
    let scenario = Scenario::new();
    let connecting = scenario.connect("race");
    scenario.server.wait_for_identity_lookup();
    // Act
    let expected = scenario.duplicate_alias();
    scenario.server.release.send(()).unwrap();
    let connected = connecting.finish();
    // Assert
    assert_eq!(connected.status.code(), Some(2));
    assert_eq!(
        serde_json::to_value(read_registry(scenario.directory.path()).unwrap()).unwrap(),
        expected
    );
    assert!(!scenario.directory.path().join("credentials.toml").exists());
}
