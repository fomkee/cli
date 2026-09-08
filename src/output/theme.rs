use anstyle::{AnsiColor, Color, Style};

pub(super) const TITLE: Style = Style::new().bold();
pub(super) const SECTION: Style = TITLE.fg_color(Some(Color::Ansi(AnsiColor::Cyan)));
pub(super) const SUCCESS: Style = TITLE.fg_color(Some(Color::Ansi(AnsiColor::Green)));
pub(super) const WARNING: Style = TITLE.fg_color(Some(Color::Ansi(AnsiColor::Yellow)));
pub(super) const FAILURE: Style = TITLE.fg_color(Some(Color::Ansi(AnsiColor::Red)));
pub(super) const HINT: Style = Style::new().dimmed();
pub(super) const ACTIVE: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));

pub(super) enum Verdict {
    Success,
    Failure,
    Warning,
    Neutral,
}

pub(super) fn verdict(status: Verdict) -> Style {
    match status {
        Verdict::Success => SUCCESS,
        Verdict::Failure => FAILURE,
        Verdict::Warning => WARNING,
        Verdict::Neutral => TITLE,
    }
}
