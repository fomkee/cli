use super::*;
use crate::wire::Response;
use serde_json::json;
fn target() -> Destination {
    Response::<Destination>::parse(json!({"id":"00000000-0000-0000-0000-000000000001", "workspace_id":"00000000-0000-0000-0000-000000000001",
        "name":"Ops\u{1b}[2J\u{202e}chat", "channel_type":"webhook", "state":{"state":"available"}, "created_at":"2026-09-15T00:00:00Z", "updated_at":"2026-09-15T00:00:00Z",
        "config":{"webhook_url":"https://example.com/?token=secret-token", "future_private_key":"private-value"}})).unwrap().data().clone()
}
fn rendered(width: u16, color: bool) -> String {
    let mut ui = Ui::new(width, color);
    destination(&mut ui, &target(), "Alert destination", true);
    ui.finish()
}
#[test]
fn test_details_hide_secret_urls_and_unrecognized_configuration() {
    let output = rendered(120, false);
    assert!(!output.contains("secret-token") && !output.contains("private-value"));
    assert!(output.contains("content hidden"));
}
#[test]
fn test_plain_narrow_output_keeps_identifiers_and_escapes_remote_controls() {
    let output = rendered(40, false);
    assert!(output.contains("00000000-0000-0000-0000-000000000001"));
    assert!(!output.contains('\u{1b}') && !output.contains('\u{202e}'));
}
#[test]
fn test_colored_wide_output_styles_headings_without_remote_terminal_controls() {
    let output = rendered(120, true);
    assert!(output.contains("\u{1b}[1m"));
    assert!(!output.contains("\u{1b}[2J"));
}
#[test]
fn test_empty_destination_page_gives_creation_hint() {
    let mut ui = Ui::new(80, false);
    list(
        &mut ui,
        &Page {
            items: vec![],
            next_cursor: None,
        },
        false,
    );
    assert!(ui.finish().contains("destination create --file"));
}
