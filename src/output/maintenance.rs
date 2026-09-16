use super::alerting::continuation;
use super::format::{text, timestamp};
use super::layout::Ui;
use crate::dto::alerting::Page;
use crate::dto::maintenance::Maintenance;

pub(super) fn detail(ui: &mut Ui, window: &Maintenance, heading: &str, details: bool) {
    ui.title(heading);
    ui.fields(vec![
        ("Title".into(), text(&window.title)),
        ("Status".into(), window.state.label().into()),
        ("Maintenance ID".into(), window.id.to_string()),
        ("Start".into(), timestamp(&window.scheduled_start)),
        ("End".into(), timestamp(&window.scheduled_end)),
    ]);
    if let Some(description) = &window.description {
        ui.line(&text(description));
    }
    ui.section("Affected monitors");
    for monitor in &window.monitors {
        ui.line(&monitor.to_string());
    }
    if details {
        ui.fields(vec![("Created".into(), timestamp(&window.created_at))]);
    }
}
pub(super) fn list(ui: &mut Ui, page: &Page<Maintenance>, details: bool) {
    ui.title(&format!("Maintenance · {} on this page", page.items.len()));
    if page.items.is_empty() {
        ui.line("No maintenance windows found.");
        ui.hint("Schedule one with fomkeecli maintenance create --help");
    } else if ui.narrow() || details {
        for item in &page.items {
            ui.line("");
            detail(ui, item, "Maintenance", details);
        }
    } else {
        ui.table(
            &["TITLE", "STATUS", "START", "END", "MAINTENANCE ID"],
            page.items
                .iter()
                .map(|item| {
                    vec![
                        text(&item.title),
                        item.state.label().into(),
                        timestamp(&item.scheduled_start),
                        timestamp(&item.scheduled_end),
                        item.id.to_string(),
                    ]
                })
                .collect(),
            Some(4),
        );
    }
    continuation(ui, page.next_cursor.as_deref());
}

#[cfg(test)]
mod tests;
