pub(super) struct Skill {
    pub(super) name: &'static str,
    pub(super) files: &'static [(&'static str, &'static [u8])],
}

pub(super) const SKILLS: &[Skill] = &[
    Skill {
        name: "managing-fomkee-monitors",
        files: &[
            (
                "SKILL.md",
                include_bytes!("../../skills/managing-fomkee-monitors/SKILL.md"),
            ),
            (
                "agents/openai.yaml",
                include_bytes!("../../skills/managing-fomkee-monitors/agents/openai.yaml"),
            ),
            (
                "references/authentication.md",
                include_bytes!(
                    "../../skills/managing-fomkee-monitors/references/authentication.md"
                ),
            ),
            (
                "references/monitor-types.md",
                include_bytes!("../../skills/managing-fomkee-monitors/references/monitor-types.md"),
            ),
            (
                "references/safety.md",
                include_bytes!("../../skills/managing-fomkee-monitors/references/safety.md"),
            ),
        ],
    },
    Skill {
        name: "writing-fomkee-check-functions",
        files: &[
            (
                "SKILL.md",
                include_bytes!("../../skills/writing-fomkee-check-functions/SKILL.md"),
            ),
            (
                "agents/openai.yaml",
                include_bytes!("../../skills/writing-fomkee-check-functions/agents/openai.yaml"),
            ),
        ],
    },
];
