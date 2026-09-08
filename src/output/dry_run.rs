use super::format::{milliseconds, text, timestamp};
use super::layout::Ui;
use super::theme::Verdict;
use crate::dto::dry_run::{Capture, DryRun, Http, Status};

fn verdict(status: Status) -> Verdict {
    match status {
        Status::Passed => Verdict::Success,
        Status::Failed => Verdict::Failure,
        Status::ExecutionUnavailable => Verdict::Warning,
        Status::NotRun => Verdict::Neutral,
    }
}

pub(super) fn render(ui: &mut Ui, value: &DryRun, details: bool) {
    ui.verdict(
        &format!("Dry run · {}", value.status().label()),
        verdict(value.status()),
    );
    let http = value.http();
    ui.section_fields(
        "HTTP check",
        vec![
            ("Result", Some(http.status.label().into())),
            ("Summary", http.summary.as_deref().map(text)),
        ],
    );
    if let Some(request) = &http.request {
        ui.section_fields(
            "Request",
            vec![
                ("Method", Some(text(&request.method))),
                ("URL", Some(text(&request.url))),
            ],
        );
    }
    if let Some(response) = &http.response {
        capture(ui, "Response", response, true);
    }
    if let Some(validation) = &http.validation {
        ui.section_fields(
            "Validation",
            vec![
                ("Result", Some(validation.status.label().into())),
                ("Message", validation.message.as_deref().map(text)),
                ("Reason", validation.reason.as_deref().map(text)),
            ],
        );
    }
    match value {
        DryRun::Http(_) => {}
        DryRun::Function { function, .. } => {
            ui.section_fields(
                "Function",
                vec![
                    ("Result", Some(function.status.label().into())),
                    ("Summary", function.summary.as_deref().map(text)),
                    ("Reason", function.reason.as_deref().map(text)),
                ],
            );
            if !function.logs.is_empty() {
                ui.section("Console");
                for log in &function.logs {
                    ui.line(&format!(
                        "  [{}] {}",
                        text(&log.level),
                        text(log.message.expose())
                    ));
                }
            }
            if details && let Some(usage) = &function.resource_usage {
                ui.object("Execution resources", usage);
            }
        }
    }
    if details {
        attempts(ui, http);
        ui.hint("Request/response bodies and header values are hidden. --json returns the full API result.");
    } else {
        ui.hint("Use --details for attempts and execution metadata.");
    }
}

fn capture(ui: &mut Ui, heading: &str, capture: &Capture, latency: bool) {
    let (kind, status, elapsed, message) = match capture {
        Capture::Http {
            status, latency_ms, ..
        } => ("http", Some(status.to_string()), *latency_ms, None),
        Capture::ConnectionError {
            message,
            latency_ms,
        } => ("connection_error", None, *latency_ms, Some(text(message))),
        Capture::Timeout { latency_ms, .. } => ("timeout", None, *latency_ms, None),
    };
    ui.section_fields(
        heading,
        vec![
            ("Kind", Some(kind.into())),
            ("HTTP status", status),
            ("Latency", latency.then(|| milliseconds(elapsed))),
            ("Message", message),
        ],
    );
}

fn attempts(ui: &mut Ui, http: &Http) {
    for attempt in &http.attempts {
        ui.section_fields(
            &format!("Attempt {}", attempt.attempt_number),
            vec![
                ("Sent", Some(timestamp(&attempt.sent_at))),
                ("Completed", Some(timestamp(&attempt.completed_at))),
                ("Latency", Some(milliseconds(attempt.latency_ms))),
            ],
        );
        capture(ui, "Result", &attempt.response, false);
    }
    if let Some(Capture::Http {
        body: Some(body), ..
    }) = &http.response
    {
        ui.section_fields(
            "Response body",
            vec![
                ("Type", body.content_type.as_deref().map(text)),
                ("Original bytes", Some(body.original_size.to_string())),
                ("Stored bytes", Some(body.stored_size.to_string())),
                (
                    "Truncated",
                    Some(if body.truncated {
                        "Yes".into()
                    } else {
                        "No".into()
                    }),
                ),
            ],
        );
    }
    if let Some(message) = &http.message {
        ui.line(&text(message));
    }
}
