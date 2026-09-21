use super::format::{duration, label, text};
use super::layout::Ui;
use super::theme::Verdict;
use crate::dto::Entitlements;
use crate::result::WorkspaceAction;
use crate::workspace::output::{
    AuthStatus, ConfigInfo, ConfigPaths, CredentialSource, Disconnected, ProfileOutput,
    WorkspaceList,
};

fn yes(value: bool) -> String {
    if value { "Yes".into() } else { "No".into() }
}

pub(super) fn list(ui: &mut Ui, value: &WorkspaceList, details: bool) {
    ui.title(&format!(
        "Workspaces · {} connected",
        value.workspaces.len()
    ));
    if value.workspaces.is_empty() {
        ui.line("No workspaces connected.");
        ui.hint("Run fomkeecli workspace connect ALIAS.");
    } else if ui.narrow() || details {
        for item in &value.workspaces {
            ui.section(&item.alias.to_string());
            profile(ui, item, details);
        }
    } else {
        ui.line("");
        ui.table(
            &["ALIAS", "NAME", "ACTIVE", "CREDENTIALS"],
            value
                .workspaces
                .iter()
                .map(|item| {
                    vec![
                        item.alias.to_string(),
                        text(&item.name),
                        yes(item.active),
                        CredentialSource::from(item.credential_store).label().into(),
                    ]
                })
                .collect(),
            None,
        );
        ui.hint("Use --details for API origins and workspace IDs.");
    }
}

fn profile(ui: &mut Ui, value: &ProfileOutput, details: bool) {
    ui.fields(vec![
        ("Workspace".into(), text(&value.name)),
        ("Alias".into(), value.alias.to_string()),
        ("Active".into(), yes(value.active)),
        ("API URL".into(), text(&value.api_url)),
        (
            "Credentials".into(),
            CredentialSource::from(value.credential_store)
                .label()
                .into(),
        ),
    ]);
    if details {
        ui.section_fields(
            "Identity",
            vec![("Workspace ID", Some(value.workspace_id.to_string()))],
        );
    }
}

pub(super) fn action(ui: &mut Ui, value: &ProfileOutput, action: WorkspaceAction, details: bool) {
    ui.verdict(
        &format!("{} workspace “{}”.", action.label(), value.alias),
        Verdict::Success,
    );
    profile(ui, value, details);
}

pub(super) fn disconnected(ui: &mut Ui, value: &Disconnected) {
    ui.verdict(
        &format!("Disconnected workspace “{}”.", value.disconnected),
        Verdict::Success,
    );
    ui.line("The API key remains valid on the server.");
    ui.fields(vec![(
        "Active workspace".into(),
        value
            .active
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "Not set".into()),
    )]);
}

pub(super) fn auth(ui: &mut Ui, value: &AuthStatus, details: bool) {
    ui.verdict("Authenticated", Verdict::Success);
    ui.section_fields(
        "Connection",
        vec![
            (
                "Alias",
                value.connection.alias.as_ref().map(ToString::to_string),
            ),
            ("API URL", Some(text(&value.connection.api_url))),
            (
                "Credentials",
                Some(value.connection.credential_source.label().into()),
            ),
        ],
    );
    let session = value.session.data();
    ui.section_fields(
        "Identity",
        vec![
            ("Workspace ID", Some(session.workspace_id.to_string())),
            ("Caller type", session.caller_type.as_deref().map(text)),
            ("User ID", session.user_id.as_deref().map(text)),
            ("API key ID", session.api_key_id.as_deref().map(text)),
            (
                "Role",
                session.role.as_deref().map(|role| {
                    if role == "ApiKey" {
                        "API key".into()
                    } else {
                        text(role)
                    }
                }),
            ),
        ],
    );
    if details && let Some(capabilities) = &session.capabilities {
        ui.object("Capabilities", capabilities);
    }
}

pub(super) fn paths(ui: &mut Ui, value: &ConfigPaths) {
    ui.title("Configuration paths");
    ui.section_fields(
        "Files",
        vec![
            ("Directory", Some(text(&value.directory.to_string_lossy()))),
            (
                "Workspaces",
                Some(text(&value.workspaces.to_string_lossy())),
            ),
            (
                "Credentials",
                Some(text(&value.credentials.to_string_lossy())),
            ),
        ],
    );
    ui.hint("Override with --config-dir PATH or FOMKEE_CONFIG_DIR. Files may not exist yet.");
}

pub(super) fn config(ui: &mut Ui, value: &ConfigInfo) {
    ui.title("Effective configuration");
    let (directory, workspace) = match value {
        ConfigInfo::Saved {
            directory,
            alias,
            api_url,
            workspace_id,
            name,
            credential_source,
        } => {
            ui.section_fields(
                "Connection",
                vec![
                    ("Workspace", Some(text(name))),
                    ("Alias", Some(alias.to_string())),
                    ("API URL", Some(text(api_url))),
                    (
                        "Credentials",
                        Some(CredentialSource::from(*credential_source).label().into()),
                    ),
                ],
            );
            (directory, Some(workspace_id.to_string()))
        }
        ConfigInfo::Environment {
            directory,
            api_url,
            credential_source,
            ..
        } => {
            ui.section_fields(
                "Connection",
                vec![
                    ("API URL", Some(text(api_url))),
                    ("Credentials", Some(credential_source.label().into())),
                ],
            );
            (directory, None)
        }
        ConfigInfo::Unselected {
            directory,
            connection,
        } => {
            ui.section_fields("Connection", vec![("Status", Some(text(connection)))]);
            (directory, None)
        }
    };
    ui.section_fields(
        "Local configuration",
        vec![
            ("Directory", Some(text(&directory.to_string_lossy()))),
            ("Workspace ID", workspace),
        ],
    );
    ui.hint("No credentials were loaded. Run fomkeecli auth status to verify access.");
}

pub(super) fn entitlement(ui: &mut Ui, value: &Entitlements, details: bool) {
    ui.title("Workspace entitlements");
    {
        let plan = &value.plan;
        ui.section_fields(
            "Plan",
            vec![
                ("Name", plan.display_name.as_deref().map(text)),
                ("Code", plan.code.as_deref().map(text)),
                ("Status", plan.status.as_deref().map(label)),
            ],
        );
    }
    if let Some(standing) = &value.grant_standing {
        ui.object("Access standing", standing);
    }
    {
        let plan = &value.plan;
        let limits = &plan.revision.entitlements;
        ui.section("Monitoring");
        if let Some(seconds) = limits.monitoring.minimum_check_interval_seconds {
            ui.fields(vec![("Minimum check interval".into(), duration(seconds))]);
        }
        for (key, value) in &limits.monitoring.other {
            ui.object(&label(key), value);
        }
        for (name, limits) in &limits.sections {
            ui.object(&label(name), limits);
        }
        if details {
            ui.section("Plan metadata");
            for (key, value) in &value.metadata {
                ui.object(&label(key), value);
            }
            for (key, value) in &plan.metadata {
                ui.object(&label(key), value);
            }
            for (key, value) in &plan.revision.metadata {
                ui.object(&label(key), value);
            }
        }
    }
    if !details {
        ui.hint("Use --details for plan revision and assignment metadata.");
    }
}
