use super::alerting::continuation;
use super::format::{text, timestamp};
use super::layout::Ui;
use crate::dto::alerting::Page;
use crate::dto::incidents::{EventKind, Incident, IncidentEvent, PublishedNote};

pub(super) fn detail(ui: &mut Ui, incident: &Incident, details: bool) {
    ui.title("Incident");
    ui.fields(vec![
        ("Status".into(), incident.state.label().into()),
        ("Monitor".into(), text(&incident.monitor_name)),
        ("Incident ID".into(), incident.id.to_string()),
        ("Monitor ID".into(), incident.monitor_id.to_string()),
        ("Opened".into(), timestamp(&incident.created_at)),
    ]);
    if let Some(resolved) = &incident.resolved_at {
        ui.fields(vec![("Resolved".into(), timestamp(resolved))]);
    }
    if details {
        ui.fields(vec![
            ("Monitor type".into(), text(&incident.monitor_type)),
            ("Regressions".into(), incident.regression_count.to_string()),
            (
                "Confirming nodes".into(),
                text(&incident.confirming_nodes.join(", ")),
            ),
        ]);
    }
}
pub(super) fn list(ui: &mut Ui, page: &Page<Incident>, details: bool) {
    ui.title(&format!("Incidents · {} on this page", page.items.len()));
    if page.items.is_empty() {
        ui.line("No incidents found.");
        ui.hint("Check your monitors with fomkeecli monitor list");
    } else if ui.narrow() || details {
        for item in &page.items {
            ui.line("");
            detail(ui, item, details);
        }
    } else {
        ui.table(
            &["MONITOR", "STATUS", "OPENED", "INCIDENT ID"],
            page.items
                .iter()
                .map(|item| {
                    vec![
                        text(&item.monitor_name),
                        item.state.label().into(),
                        timestamp(&item.created_at),
                        item.id.to_string(),
                    ]
                })
                .collect(),
            Some(3),
        );
    }
    continuation(ui, page.next_cursor.as_deref());
}
pub(super) fn timeline(ui: &mut Ui, page: &Page<IncidentEvent>) {
    ui.title(&format!(
        "Incident timeline · {} on this page",
        page.items.len()
    ));
    if page.items.is_empty() {
        ui.line("No events on this page.");
    }
    for event in &page.items {
        ui.line("");
        ui.section(event.event.label());
        ui.fields(vec![
            ("Time".into(), timestamp(&event.occurred_at)),
            ("Event ID".into(), event.id.to_string()),
        ]);
        if let EventKind::StatusUpdatePosted { message, .. } = &event.event {
            ui.line(&text(message));
        }
    }
    continuation(ui, page.next_cursor.as_deref());
}
pub(super) fn published(ui: &mut Ui, note: &PublishedNote) {
    ui.title("Published incident note");
    ui.fields(vec![
        ("Event ID".into(), note.id.to_string()),
        ("Published".into(), timestamp(&note.occurred_at)),
    ]);
    ui.line(&text(&note.message));
}

#[cfg(test)]
mod tests;
