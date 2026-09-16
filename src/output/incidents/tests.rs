use super::*;
use crate::wire::Response;
use serde_json::json;

const ID: &str = "00000000-0000-0000-0000-000000000005";
fn incident() -> Incident {
    Response::<Incident>::parse(json!({"id": ID, "monitor_id": ID, "workspace_id": ID, "state": "regressed", "monitor_name": "API\u{1b}[2J\u{202e}", "monitor_type": "http", "created_at": "2026-09-16T10:00:00Z", "resolved_at": null, "confirming_nodes": ["eu-1"], "regression_count": 1})).unwrap().data().clone()
}
fn rendered(width: u16, color: bool, details: bool) -> String {
    let mut ui = Ui::new(width, color);
    list(
        &mut ui,
        &Page {
            items: vec![incident()],
            next_cursor: Some("next/page".into()),
        },
        details,
    );
    ui.finish()
}
fn assert_safe_identifiable_page(output: &str) {
    assert!(
        output.contains(ID) && output.contains("Regressed") && output.contains("--after next/page")
    );
    assert!(!output.contains("\u{1b}[2J") && !output.contains('\u{202e}'));
}
#[test]
fn test_narrow_incidents_keep_status_ids_and_pagination() {
    // Act
    let output = rendered(40, false, false);
    // Assert
    assert_safe_identifiable_page(&output);
}
#[test]
fn test_wide_colored_incidents_escape_server_controls() {
    // Act
    let output = rendered(120, true, false);
    // Assert
    assert_safe_identifiable_page(&output);
}
#[test]
fn test_details_show_regression_history() {
    // Act
    let output = rendered(80, false, true);
    // Assert
    assert!(
        output.contains("Regressions") && output.contains("eu-1") && !output.contains('\u{1b}')
    );
}
#[test]
fn test_empty_incidents_offer_monitor_inspection() {
    // Arrange
    let mut ui = Ui::new(80, false);
    // Act
    list(
        &mut ui,
        &Page {
            items: vec![],
            next_cursor: None,
        },
        false,
    );
    // Assert
    assert!(ui.finish().contains("fomkeecli monitor list"));
}
#[test]
fn test_timeline_displays_escaped_public_note_text() {
    // Arrange
    let mut ui = Ui::new(40, false);
    let event = IncidentEvent {
        id: ID.parse().unwrap(),
        occurred_at: "2026-09-16T10:00:00Z".into(),
        event: EventKind::StatusUpdatePosted {
            message: "Working on it\u{1b}[2J".into(),
            posted_by: ID.into(),
        },
    };
    // Act
    timeline(
        &mut ui,
        &Page {
            items: vec![event],
            next_cursor: None,
        },
    );
    // Assert
    let output = ui.finish();
    assert!(output.contains("Working on it") && output.contains(ID) && !output.contains('\u{1b}'));
}
