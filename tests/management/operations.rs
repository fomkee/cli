use super::management_support::{MONITOR, Step, WS, run, succeeded};
use axum::http::StatusCode;
use serde_json::{Value, json};

const INCIDENT: &str = "00000000-0000-0000-0000-000000000005";
const EVENT: &str = "00000000-0000-0000-0000-000000000006";
const MAINTENANCE: &str = "00000000-0000-0000-0000-000000000007";
const START: &str = "2030-10-01T09:00:00+02:00";
const END: &str = "2030-10-01T10:00:00+02:00";
fn incident() -> Value {
    json!({"id": INCIDENT, "monitor_id": MONITOR, "workspace_id": WS, "state": "open", "monitor_name": "API", "monitor_type": "http", "created_at": START, "resolved_at": null, "confirming_nodes": ["eu-1"], "regression_count": 0, "future": true})
}
fn note() -> Value {
    json!({"id": EVENT, "type": "status_update_posted", "occurred_at": START, "posted_by": WS, "message": "Investigating the outage.", "future": true})
}
fn settings() -> Value {
    json!({"title": "Database upgrade", "description": null, "scheduled_start": START, "scheduled_end": END, "monitors": [MONITOR]})
}
fn window() -> Value {
    let mut value = settings();
    value.as_object_mut().unwrap().extend(json!({"id": MAINTENANCE, "workspace_id": WS, "state": "scheduled", "created_at": START, "created_by": WS, "future": true}).as_object().unwrap().clone());
    value
}
fn page(item: Value) -> Value {
    json!({"items": [item], "next_cursor": "next/page", "extension": true})
}

#[path = "operations/incidents.rs"]
mod incidents;
#[path = "operations/maintenance.rs"]
mod maintenance;
