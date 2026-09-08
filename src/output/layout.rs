use crate::dto::{Lifecycle, MonitorKind};
use anstyle::Style;
use comfy_table::{ColumnConstraint, ContentArrangement, Table, Width, presets::NOTHING};
use serde_json::Value;
use unicode_width::UnicodeWidthStr;

use super::format::{formatted, label, scalar, sensitive};
use super::theme;

pub(super) struct Ui {
    width: u16,
    color: bool,
    lines: Vec<String>,
}

impl Ui {
    pub(super) fn new(width: u16, color: bool) -> Self {
        Self {
            width: width.clamp(20, 300),
            color,
            lines: Vec::new(),
        }
    }

    pub(super) fn narrow(&self) -> bool {
        self.width < 80
    }

    pub(super) fn page_width(&mut self) {
        self.width = self.width.saturating_sub(2);
    }

    pub(super) fn section_fields(&mut self, heading: &str, fields: Vec<(&str, Option<String>)>) {
        let fields: Vec<_> = fields
            .into_iter()
            .filter_map(|(name, value)| {
                value
                    .filter(|text| !text.is_empty())
                    .map(|value| (name.into(), value))
            })
            .collect();
        if !fields.is_empty() {
            self.section(heading);
            self.fields(fields);
        }
    }

    fn styled(&self, value: &str, style: Style) -> String {
        if self.color {
            format!("{style}{value}{style:#}")
        } else {
            value.into()
        }
    }

    pub(super) fn title(&mut self, title: &str) {
        self.styled_lines(title, theme::TITLE);
    }

    pub(super) fn line(&mut self, value: &str) {
        self.lines.extend(wrap(value, usize::from(self.width), ""));
    }

    fn styled_lines(&mut self, value: &str, style: Style) {
        for line in wrap(value, usize::from(self.width), "") {
            self.lines.push(self.styled(&line, style));
        }
    }

    pub(super) fn section(&mut self, name: &str) {
        self.lines.push(String::new());
        self.styled_lines(name, theme::SECTION);
    }

    pub(super) fn hint(&mut self, value: &str) {
        self.lines.push(String::new());
        self.styled_lines(value, theme::HINT);
    }

    pub(super) fn verdict(&mut self, value: &str, status: theme::Verdict) {
        self.styled_lines(value, theme::verdict(status));
    }

    pub(super) fn fields(&mut self, fields: Vec<(String, String)>) {
        if fields.is_empty() {
            return;
        }
        let max_label = fields.iter().map(|(key, _)| key.width()).max().unwrap_or(0);
        for (key, value) in fields {
            let padding = " ".repeat(max_label.saturating_sub(key.width()));
            if self.narrow() && max_label > 18 {
                self.line(&format!("  {key}"));
                self.lines
                    .extend(wrap(&value, usize::from(self.width), "    "));
            } else {
                let prefix = format!("  {key}{padding}  ");
                if key.ends_with("ID")
                    || matches!(
                        key.as_str(),
                        "Directory" | "Workspaces" | "Credentials" | "URL" | "API URL"
                    )
                {
                    self.lines.push(format!("{prefix}{value}"));
                } else {
                    self.lines
                        .extend(wrap(&value, usize::from(self.width), &prefix));
                }
            }
        }
    }

    pub(super) fn table(
        &mut self,
        headings: &[&str],
        rows: Vec<Vec<String>>,
        id_column: Option<usize>,
    ) {
        let mut table = Table::new();
        table
            .load_style(NOTHING)
            .set_width(self.width)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(headings.to_vec())
            .add_rows(rows);
        if let Some(index) = id_column
            && let Some(column) = table.column_mut(index)
        {
            column.set_constraint(ColumnConstraint::LowerBoundary(Width::Fixed(38)));
        }
        let mut lines = table.lines();
        if let Some(header) = lines.next() {
            self.lines
                .push(self.styled(header.trim_end(), theme::TITLE));
        }
        self.lines
            .extend(lines.map(|line| line.trim_end().to_owned()));
    }

    pub(super) fn object(&mut self, heading: &str, value: &Value) {
        self.section(heading);
        self.object_fields(value, 0);
    }

    fn object_fields(&mut self, value: &Value, depth: usize) {
        if depth >= 6 {
            self.line("  Further nesting available with --json.");
            return;
        }
        match value {
            Value::Object(object) => {
                let mut fields = Vec::new();
                for (key, value) in object {
                    if matches!(key.as_str(), "headers" | "body" | "js_source" | "auth") {
                        fields.push((
                            label(key),
                            "Content hidden; use --json for the API result".into(),
                        ));
                    } else if sensitive(key) || !value.is_object() && !value.is_array() {
                        fields.push((label(key), formatted(key, value)));
                    } else {
                        self.fields(fields);
                        fields = Vec::new();
                        self.line(&format!("  {}", label(key)));
                        self.object_fields(value, depth.saturating_add(1));
                    }
                }
                self.fields(fields);
            }
            Value::Array(items)
                if items
                    .iter()
                    .all(|item| !item.is_array() && !item.is_object()) =>
            {
                let value = if items.is_empty() {
                    "None".into()
                } else {
                    items.iter().map(scalar).collect::<Vec<_>>().join(", ")
                };
                self.line(&format!("  {value}"));
            }
            Value::Array(items) => {
                if items.is_empty() {
                    self.line("  None");
                }
                for (index, item) in items.iter().enumerate() {
                    self.line(&format!("  [{}]", index.saturating_add(1)));
                    self.object_fields(item, depth.saturating_add(1));
                }
            }
            value @ (Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)) => {
                self.line(&format!("  {}", scalar(value)))
            }
        }
    }

    pub(super) fn finish(self) -> String {
        self.lines.join("\n").trim_end().to_owned()
    }

    pub(super) fn finish_page(self) -> String {
        let content = self
            .finish()
            .lines()
            .map(|line| {
                if line.is_empty() {
                    String::new()
                } else {
                    format!("  {line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("\n{content}\n")
    }

    pub(super) fn lifecycle(&mut self, state: Lifecycle, kind: MonitorKind) {
        let dot = self.styled(
            "●",
            match state {
                Lifecycle::Active => theme::ACTIVE,
                Lifecycle::Paused | Lifecycle::Disabled => theme::HINT,
            },
        );
        self.line(&format!("{dot} {} · {}", state.label(), kind.label()));
    }

    pub(super) fn reference(&mut self, label: &str, value: &str, secondary: Option<&str>) {
        let secondary = secondary
            .map(|value| format!(" ({value})"))
            .unwrap_or_default();
        self.lines.push(format!(
            "  {label:<10} {value}{}",
            self.styled(&secondary, theme::HINT)
        ));
    }

    pub(super) fn subdued_reference(&mut self, label: &str, value: &str) {
        self.lines
            .push(format!("  {label:<10} {}", self.styled(value, theme::HINT)));
    }
}

fn wrap(value: &str, width: usize, prefix: &str) -> Vec<String> {
    if prefix.width().saturating_add(value.width()) <= width {
        return vec![format!("{prefix}{value}")];
    }
    let indent = " ".repeat(prefix.width().min(24));
    let mut line = prefix.to_owned();
    let mut lines = Vec::new();
    for word in value.split_whitespace() {
        let has_value = !line.trim().is_empty() && line != prefix && line != indent;
        if has_value && line.width().saturating_add(1).saturating_add(word.width()) > width {
            lines.push(line);
            line = indent.clone();
        } else if has_value {
            line.push(' ');
        }
        line.push_str(word);
    }
    lines.push(line);
    lines
}
