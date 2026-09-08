use super::format::text;
use super::layout::Ui;
use super::theme::Verdict;
use crate::skill::ExportedSkills;

pub(super) fn export(ui: &mut Ui, value: &ExportedSkills) {
    ui.verdict("Exported Fomkee skills", Verdict::Success);
    ui.section_fields(
        "Bundle",
        vec![
            ("Directory", Some(text(&value.directory.to_string_lossy()))),
            ("CLI version", Some(value.cli_version.into())),
        ],
    );
    ui.section("Skills");
    for skill in &value.skills {
        ui.line(&format!("  {}", skill.name));
    }
    ui.hint("Review each SKILL.md and its supporting files, then import the complete skill folder using your agent’s native workflow.");
    ui.line("No agent configuration was changed.");
}
