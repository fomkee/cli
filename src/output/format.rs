use chrono::{DateTime, Utc};
use serde_json::Value;

pub(super) fn text(value: &str) -> String {
    value.chars().flat_map(|c| {
        if c.is_control() || matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') {
            c.escape_default().collect::<Vec<_>>()
        } else {
            vec![c]
        }
    }).collect()
}

pub(super) fn label(key: &str) -> String {
    match key {
        "id" => "ID".into(),
        "api_url" => "API URL".into(),
        "api" => "API".into(),
        "url" => "URL".into(),
        "js_source" => "JavaScript".into(),
        _ => {
            let key = key
                .strip_suffix("_seconds")
                .or_else(|| key.strip_suffix("_secs"))
                .or_else(|| key.strip_suffix("_ms"))
                .unwrap_or(key);
            let words = text(&key.replace('_', " "));
            let mut chars = words.chars();
            chars
                .next()
                .map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()))
                .unwrap_or_default()
        }
    }
}

pub(super) fn scalar(value: &Value) -> String {
    match value {
        Value::String(value) => text(value),
        Value::Null => "Not set".into(),
        Value::Bool(value) => if *value { "Yes" } else { "No" }.into(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => format!("{} items", values.len()),
        Value::Object(_) => "Configured".into(),
    }
}

pub(super) fn formatted(key: &str, value: &Value) -> String {
    if sensitive(key) && !value.is_null() {
        return "[redacted]".into();
    }
    if let Some(seconds) = value.as_u64()
        && (key.ends_with("_secs") || key.ends_with("_seconds"))
    {
        return duration(seconds);
    }
    if let Some(ms) = value.as_u64()
        && key.ends_with("_ms")
    {
        return if ms != 0 && ms.is_multiple_of(1000) {
            duration(ms.checked_div(1000).unwrap_or(0))
        } else {
            format!("{ms} ms")
        };
    }
    if let Some(string) = value.as_str() {
        return match key {
            "role" if string == "ApiKey" => "API key".into(),
            "config_type" | "type" if string == "http" => "HTTP".into(),
            "state" | "status" | "config_type" | "type" | "credential_store"
            | "credential_source" => label(string),
            "expected_status" if string == "any_success" => "Any successful HTTP status".into(),
            "created_at" | "updated_at" | "sent_at" | "completed_at" | "received_at" => {
                timestamp(string)
            }
            _ => text(string),
        };
    }
    if key == "tags"
        && let Some(items) = value.as_array()
    {
        return items.iter().map(scalar).collect::<Vec<_>>().join(", ");
    }
    scalar(value)
}

pub(super) fn sensitive(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("secret")
        || key.contains("password")
        || key.contains("token")
        || matches!(
            key.as_str(),
            "authorization" | "cookie" | "set-cookie" | "api_key" | "content_base64"
        )
}

pub(super) fn duration(seconds: u64) -> String {
    if seconds == 0 {
        return "0 seconds".into();
    }
    let mut remaining = seconds;
    let mut parts = Vec::new();
    for (unit, size) in [
        ("day", 86400),
        ("hour", 3600),
        ("minute", 60),
        ("second", 1),
    ] {
        let count = remaining.checked_div(size).unwrap_or(0);
        remaining = remaining.checked_rem(size).unwrap_or(0);
        if count != 0 {
            parts.push(format!(
                "{count} {unit}{}",
                if count == 1 { "" } else { "s" }
            ));
        }
    }
    parts.join(" ")
}

pub(super) fn milliseconds(ms: u64) -> String {
    if ms != 0 && ms.is_multiple_of(1000) {
        duration(ms.checked_div(1000).unwrap_or(0))
    } else {
        format!("{ms} ms")
    }
}

pub(super) fn timestamp(value: &str) -> String {
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(value) {
        return timestamp
            .with_timezone(&Utc)
            .format("%-d %b %Y at %H:%M:%S UTC")
            .to_string();
    }
    let readable = value.replace('T', " ");
    text(&match readable.strip_suffix('Z') {
        Some(value) => format!("{value} UTC"),
        None => readable,
    })
}
