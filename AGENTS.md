# Working on fomkeecli

Read [DESIGN.md](DESIGN.md) before changing commands, help, or output. It is the
canonical CLI UX specification for agents and human contributors. Update the
rules, examples, implementation, and regression tests together when deliberately
changing a convention; do not duplicate the rules in an agent-specific skill.

Keep this standalone crate dependent only on the public HTTP API. Do not move
server validation, plan policy, or defaults into presentation code. Never add
network requests merely to decorate output.

Use shared presentation primitives and command-specific views under
`src/output/`. Preserve explicit JSON response structure and exit codes. Test
plain and colored output, narrow and wide layouts, empty results, untrusted
text, and secret handling. Tests must not require real credentials or a keyring.

Run formatting, Clippy with warnings denied, and the standalone CLI test suite.
See [CONTRIBUTING.md](CONTRIBUTING.md) for contributor checks.

Use current stable dependency releases for new work; document justified
exceptions and migration constraints in [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md).
