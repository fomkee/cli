# Contributing

Read [DESIGN.md](DESIGN.md) before changing commands, help, or output. Update
design examples and output regression tests with intentional UX changes.

Agent skills in `skills/` are embedded in the binary for offline export. When
adding or removing skill files, update the explicit bundle in
`src/skill/bundled.rs`. The export integrity test compares every exported file
byte-for-byte with the complete source tree, preventing unnoticed omissions.

Keep changes limited to the exported CLI surface and public HTTP API contract.
The library's intended reusable surface consists of typed identifiers and
configuration, validated monitor inputs, the `FomkeeApi` port and its adapters,
wire responses, workspace storage/services, errors, and the process entry
point. Argument parsing, command dispatch, and presentation are crate-private.
Test doubles must return valid typed responses or explicit unconfigured
operation errors; successful empty JSON objects are not fixtures.

Keep target secrets out of diagnostic representations, retain typed error
sources, and match owned enums exhaustively. All production zero-panic lints
remain mandatory: never suppress them or copy panic-based upstream examples.
Validate remote workspace identity before taking the local write lock, then
reload and recheck the alias under the lock before committing local changes.

Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test --locked`. Public pull requests are reviewed publicly and imported
into the private canonical subtree by maintainers; never add private repository
credentials to public workflows.

## Releases

In `fomkee/cli`, a pushed `vVERSION` tag runs the native build/test matrix and
must match `fomkeecli --version`. Only that repository publishes release assets;
pull requests and forks cannot publish through this workflow.

If a tag push does not start a run, manually run `verify-and-release` from
`main` with the existing version tag as its `tag` input. This builds that tag
through the same checks without moving it or replacing an existing release.

Assets are `fomkeecli-Linux-x86_64`, `fomkeecli-Darwin-arm64`,
`fomkeecli-Darwin-x86_64`, and `fomkeecli-Windows-x86_64.exe`, each with a
`.sha256` file. Linux uses the Ubuntu 22.04 glibc baseline. Both bundled skills
are also attached as versioned archives with checksums. Keep README download
commands aligned with these names.

Publishing waits for all builds and skill packaging, verifies checksums, and
requires an existing tag. Prerelease tags remain prereleases. An existing
release is not overwritten; inspect a failed publication before retrying.
