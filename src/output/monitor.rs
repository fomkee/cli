mod settings;
use super::context::DisplayContext;
use super::format::{duration, milliseconds, text, timestamp};
use super::layout::Ui;
use super::theme::Verdict;
use crate::dto::{CreatedMonitor, Monitor, MonitorConfig, MonitorPage, Schedule};
use crate::result::MonitorAction;
use settings::settings;

pub(super) fn list(ui: &mut Ui, page: &MonitorPage, details: bool, context: &DisplayContext) {
    ui.title(&format!("Monitors · {} on this page", page.items.len()));
    if page.items.is_empty() {
        ui.line("No monitors found.");
        ui.hint("Create one with fomkeecli monitor create http https://example.com/health");
    } else if ui.narrow() || details {
        for item in &page.items {
            ui.line("");
            if details {
                detail(ui, item, true, context);
            } else {
                summary(ui, item);
                references(ui, item, context);
            }
        }
    } else {
        ui.line("");
        ui.table(
            &["NAME", "TYPE", "STATE", "ID"],
            page.items
                .iter()
                .map(|item| {
                    vec![
                        text(&item.name),
                        item.config.kind().label().into(),
                        item.state.label().into(),
                        item.id.to_string(),
                    ]
                })
                .collect(),
            Some(3),
        );
    }
    if let Some(cursor) = &page.next_cursor {
        ui.hint(&format!(
            "More monitors available. Continue with --after {}",
            text(cursor)
        ));
    }
}

fn summary(ui: &mut Ui, monitor: &Monitor) {
    ui.lifecycle(monitor.state, monitor.config.kind());
    if let Some(active) = monitor.config.active() {
        ui.line("");
        ui.line(&format!("{} {}", text(&active.method), text(&active.url)));
    }
}

pub(super) fn detail(ui: &mut Ui, monitor: &Monitor, details: bool, context: &DisplayContext) {
    summary(ui, monitor);
    checks(ui, monitor, false);
    ui.section_fields(
        "About",
        vec![
            ("Description", monitor.description.as_deref().map(text)),
            (
                "Tags",
                (!monitor.tags.is_empty()).then(|| {
                    monitor
                        .tags
                        .iter()
                        .map(|tag| text(tag))
                        .collect::<Vec<_>>()
                        .join(", ")
                }),
            ),
        ],
    );
    if details {
        settings(ui, monitor);
        history(ui, monitor);
    }
    references(ui, monitor, context);
    app_link(ui, monitor, context);
    if !details {
        ui.hint("Use --details for secondary settings and history.");
    }
}

fn checks(ui: &mut Ui, monitor: &Monitor, action: bool) {
    let mut fields = Vec::new();
    if let Some(active) = monitor.config.active() {
        if action {
            fields.push(("URL", Some(text(&active.url))));
        }
        fields.push(("Every", Some(duration(active.interval_secs))));
        if !action {
            fields.push(("Timeout", active.timeout_secs.map(duration)));
        }
    }
    match &monitor.config {
        MonitorConfig::Http {
            expected_status,
            response_time_max_ms,
            ..
        } => {
            if !action {
                fields.push((
                    "Expected",
                    expected_status.as_deref().map(|status| {
                        if status == "any_success" {
                            "Any successful HTTP status".into()
                        } else {
                            text(status)
                        }
                    }),
                ));
                fields.push(("Response limit", response_time_max_ms.map(milliseconds)));
            }
        }
        MonitorConfig::Function { .. } => {}
        MonitorConfig::Heartbeat { schedule, .. } => match schedule {
            Schedule::Interval {
                period_secs,
                grace_secs,
            } => {
                fields.push(("Expected every", Some(duration(*period_secs))));
                if !action {
                    fields.push(("Grace", grace_secs.map(duration)));
                }
            }
            Schedule::Cron {
                cron_expression,
                timezone,
                grace_secs,
            } => {
                fields.push(("Schedule (UTC)", Some(text(cron_expression))));
                if !action {
                    fields.push(("Timezone", timezone.as_deref().map(text)));
                    fields.push(("Grace", grace_secs.map(duration)));
                }
            }
        },
    }
    ui.section_fields("Checks", fields);
}

fn history(ui: &mut Ui, monitor: &Monitor) {
    if monitor.created_at.is_none() && monitor.updated_at.is_none() {
        return;
    }
    ui.section("History");
    if let Some(created) = &monitor.created_at {
        ui.fields(vec![("Created".into(), timestamp(created))]);
    }
    if let Some(updated) = &monitor.updated_at {
        if monitor.created_at.is_some() {
            ui.line("");
        }
        ui.fields(vec![("Updated".into(), timestamp(updated))]);
    }
}

fn references(ui: &mut Ui, monitor: &Monitor, context: &DisplayContext) {
    ui.section("References");
    ui.subdued_reference("Name", &text(&monitor.name));
    ui.reference("Monitor", &monitor.id.to_string(), None);
    if let Some((id, name)) = &context.workspace
        && monitor.workspace_id == *id
        && !name.is_empty()
    {
        ui.reference("Workspace", &text(name), Some(&id.to_string()));
    }
}

fn app_link(ui: &mut Ui, monitor: &Monitor, context: &DisplayContext) {
    if context.hosted {
        ui.section("In Fomkee");
        ui.line(&format!("  https://app.fomkee.dev/monitors/{}", monitor.id));
    }
}

pub(super) fn created(
    ui: &mut Ui,
    result: &CreatedMonitor,
    details: bool,
    context: &DisplayContext,
) {
    action(
        ui,
        &result.monitor,
        MonitorAction::Created,
        details,
        context,
    );
    if let Some(secret) = &result.heartbeat_secret {
        ui.section("Heartbeat secret");
        ui.verdict("Sensitive — shown once; store securely.", Verdict::Warning);
        ui.line(&text(secret.expose()));
    }
}

pub(super) fn action(
    ui: &mut Ui,
    monitor: &Monitor,
    action: MonitorAction,
    details: bool,
    context: &DisplayContext,
) {
    ui.verdict(
        &format!("{} monitor “{}”.", action.label(), text(&monitor.name)),
        Verdict::Success,
    );
    if details {
        ui.line("");
        detail(ui, monitor, true, context);
    } else {
        ui.fields(vec![
            ("Monitor ID".into(), monitor.id.to_string()),
            ("State".into(), monitor.state.label().into()),
        ]);
        checks(ui, monitor, true);
        app_link(ui, monitor, context);
    }
    if !details && !context.hosted {
        ui.hint(&format!(
            "Inspect with fomkeecli monitor get {}",
            monitor.id
        ));
    }
}
