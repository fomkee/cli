use super::format::{label, text, timestamp};
use super::layout::Ui;
use super::theme::Verdict;
use crate::dto::alerting::{
    Assignment, Destination, DestinationState, DestinationTest, Page, TestOutcome,
};
use serde_json::Value;

pub(super) fn destination(ui: &mut Ui, target: &Destination, heading: &str, details: bool) {
    ui.title(heading);
    ui.fields(vec![
        ("Name".into(), text(&target.name)),
        ("Destination ID".into(), target.id.to_string()),
        ("Channel".into(), target.channel_type.label().into()),
        ("State".into(), state(&target.state).into()),
    ]);
    if details {
        settings(ui, target);
        ui.fields(vec![
            ("Created".into(), timestamp(&target.created_at)),
            ("Updated".into(), timestamp(&target.updated_at)),
        ]);
    }
}
fn state(value: &DestinationState) -> &'static str {
    match value {
        DestinationState::Available => "Available",
        DestinationState::Unavailable { .. } => "Unavailable",
    }
}
pub(super) fn list(ui: &mut Ui, page: &Page<Destination>, details: bool) {
    ui.title(&format!(
        "Alert destinations · {} on this page",
        page.items.len()
    ));
    if page.items.is_empty() {
        ui.line("No alert destinations found.");
        ui.hint("Create one with fomkeecli destination create --file destination.json");
    } else if ui.narrow() || details {
        for item in &page.items {
            ui.line("");
            destination(ui, item, "Destination", details);
        }
    } else {
        ui.table(
            &["NAME", "CHANNEL", "STATE", "ID"],
            page.items
                .iter()
                .map(|item| {
                    vec![
                        text(&item.name),
                        item.channel_type.label().into(),
                        state(&item.state).into(),
                        item.id.to_string(),
                    ]
                })
                .collect(),
            Some(3),
        );
    }
    continuation(ui, page.next_cursor.as_deref());
}
pub(super) fn assignments(ui: &mut Ui, page: &Page<Assignment>) {
    ui.title(&format!(
        "Monitor assignments · {} on this page",
        page.items.len()
    ));
    if page.items.is_empty() {
        ui.line("No assignments found.");
        ui.hint("Attach a monitor with fomkeecli destination assign DESTINATION_ID MONITOR_ID");
    }
    for item in &page.items {
        ui.line("");
        assignment(ui, item);
    }
    continuation(ui, page.next_cursor.as_deref());
}
pub(super) fn assignment(ui: &mut Ui, item: &Assignment) {
    ui.fields(vec![
        ("Monitor ID".into(), item.monitor_id.to_string()),
        ("Destination ID".into(), item.alert_target_id.to_string()),
        ("Assignment ID".into(), item.id.to_string()),
    ]);
}
pub(super) fn test(ui: &mut Ui, value: &DestinationTest) {
    let (label, verdict) = match value.outcome {
        TestOutcome::Accepted => ("Test notification accepted by provider", Verdict::Success),
        TestOutcome::Failed => ("Test notification failed", Verdict::Failure),
        TestOutcome::Skipped => ("Test notification skipped", Verdict::Warning),
    };
    ui.verdict(label, verdict);
    ui.fields(vec![
        ("Destination ID".into(), value.alert_target_id.to_string()),
        ("Channel".into(), value.channel_kind.label().into()),
        ("Attempted".into(), timestamp(&value.attempted_at)),
    ]);
}
fn continuation(ui: &mut Ui, cursor: Option<&str>) {
    if let Some(cursor) = cursor {
        ui.hint(&format!(
            "More results available. Continue with --after {}",
            text(cursor)
        ));
    }
}

fn settings(ui: &mut Ui, target: &Destination) {
    let config = target.config.expose();
    let visible = [
        "email_address",
        "telegram_chat_id",
        "ntfy_priority",
        "pushover_priority",
    ];
    let mut fields = visible
        .into_iter()
        .filter_map(|key| {
            config
                .get(key)
                .and_then(Value::as_str)
                .map(|value| (label(key), text(value)))
        })
        .collect::<Vec<_>>();
    for key in [
        "webhook_url",
        "ntfy_topic_url",
        "telegram_bot_token_configured",
        "pushover_user_key_configured",
        "pushover_app_token_configured",
        "discord_webhook_url_configured",
        "slack_webhook_url_configured",
    ] {
        if config
            .get(key)
            .is_some_and(|value| !value.is_null() && value != false)
        {
            fields.push((label(key), "Configured; content hidden".into()));
        }
    }
    if !fields.is_empty() {
        ui.section("Settings");
        ui.fields(fields);
    }
}

#[cfg(test)]
mod tests;
