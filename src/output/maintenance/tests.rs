use super::*;
use crate::wire::Response;
use serde_json::json;

const ID: &str = "00000000-0000-0000-0000-000000000007";
fn window() -> Maintenance {
    Response::<Maintenance>::parse(json!({"id": ID, "workspace_id": ID, "title": "Upgrade\u{1b}[2J", "description": "Planned work\u{202e}", "scheduled_start": "2030-10-01T09:00:00+02:00", "scheduled_end": "2030-10-01T10:00:00+02:00", "monitors": [ID], "state": "scheduled", "created_at": "2026-09-16T10:00:00Z", "created_by": ID})).unwrap().data().clone()
}
fn rendered(width: u16, color: bool, details: bool) -> String {
    let mut ui = Ui::new(width, color);
    list(
        &mut ui,
        &Page {
            items: vec![window()],
            next_cursor: Some("more".into()),
        },
        details,
    );
    ui.finish()
}
fn assert_safe_scheduled_page(output: &str) {
    assert!(output.contains(ID) && output.contains("Scheduled") && output.contains("--after more"));
    assert!(!output.contains("\u{1b}[2J") && !output.contains('\u{202e}'));
}
#[test]
fn test_narrow_maintenance_shows_affected_monitors_and_schedule() {
    // Act
    let output = rendered(40, false, false);
    // Assert
    assert_safe_scheduled_page(&output);
}
#[test]
fn test_wide_colored_maintenance_escapes_titles() {
    // Act
    let output = rendered(120, true, false);
    // Assert
    assert_safe_scheduled_page(&output);
}
#[test]
fn test_details_show_description_and_creation_time() {
    // Act
    let output = rendered(80, false, true);
    // Assert
    assert!(
        output.contains("Planned work") && output.contains("Created") && !output.contains('\u{1b}')
    );
}
#[test]
fn test_empty_maintenance_offers_scheduling_help() {
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
    assert!(ui.finish().contains("maintenance create --help"));
}
