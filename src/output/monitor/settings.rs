use super::super::format::{label, milliseconds, text};
use super::super::layout::Ui;
use crate::dto::{Monitor, MonitorConfig, Schedule};

pub(super) fn settings(ui: &mut Ui, monitor: &Monitor) {
    if let Some(active) = monitor.config.active() {
        ui.section_fields(
            "Request",
            vec![
                (
                    "Authentication",
                    active
                        .auth
                        .as_ref()
                        .and_then(|auth| auth.configured_label())
                        .map(str::to_owned),
                ),
                (
                    "Headers",
                    (!active.headers.is_empty())
                        .then(|| format!("{} configured (values hidden)", active.headers.len())),
                ),
                (
                    "Body",
                    active
                        .body
                        .as_ref()
                        .filter(|body| !body.expose().is_null())
                        .map(|_| "Configured (content hidden)".into()),
                ),
            ],
        );
        ui.section_fields(
            "Retries",
            vec![
                (
                    "Attempts",
                    active
                        .retry_max_attempts
                        .map(|count| format!("{count} maximum")),
                ),
                (
                    "Backoff",
                    active
                        .retry_base_backoff_ms
                        .zip(active.retry_max_backoff_ms)
                        .map(|(initial, max)| {
                            format!(
                                "{} initially, {} maximum",
                                milliseconds(initial),
                                milliseconds(max)
                            )
                        }),
                ),
            ],
        );
    }
    let mut config = vec![(
        "SLA target (ppm)",
        monitor
            .sla_target_parts_per_million
            .map(|value| value.to_string()),
    )];
    match &monitor.config {
        MonitorConfig::Http { .. } => {}
        MonitorConfig::Function {
            function_id,
            js_source,
            ..
        } => {
            config.insert(0, ("Function ID", function_id.as_deref().map(text)));
            ui.section_fields("Configuration", config);
            if js_source
                .as_ref()
                .is_some_and(|source| !source.expose().is_empty())
            {
                ui.section("Function source");
                ui.line("Configured (content hidden)");
            }
            return;
        }
        MonitorConfig::Heartbeat {
            schedule,
            body_validation,
        } => {
            let kind = match schedule {
                Schedule::Interval { .. } => "interval",
                Schedule::Cron { .. } => "cron",
            };
            config.insert(0, ("Schedule type", Some(label(kind))));
            ui.section_fields("Configuration", config);
            if body_validation
                .as_ref()
                .is_some_and(|body| !body.expose().is_null())
            {
                ui.section("Body validation");
                ui.line("Configured (content hidden)");
            }
            return;
        }
    }
    ui.section_fields("Configuration", config);
}
