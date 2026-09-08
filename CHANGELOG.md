# Changelog

## 0.1.1

- Fixed the skill-export test's path expectation for macOS and symlinked
  working directories; exported paths must be absolute and identify the same
  destination. Runtime behavior and zero-panic checks are unchanged.
- Added manual release builds for existing tags without moving them.

## 0.1.0

- Added offline `skill export DIRECTORY`: exports both bundled skill folders
  for review and native harness import, without additional dependencies,
  credential access, agent configuration changes, or overwriting destinations.
- Refined monitor pages with a lifecycle-first header, breathing room, subdued
  names in References, saved workspace names, readable History, and hosted
  **In Fomkee** links. Hide empty settings, creator UUIDs, and repeated file
  fallback warnings. JSON remains unchanged.
- Updated comfy-table, reqwest, TOML, and the Keyring core/platform-store stack;
  documented credential compatibility and the upstream generic-array pin.

- Added a shared CLI design specification and agent instructions, semantic
  colors, grouped details, responsive tables, and concise action confirmations.
  Global `--details` and `--color auto|always|never` preserve explicit JSON
  output. Human views escape terminal controls and summarize sensitive settings.

- Unified singular commands (`monitor`, `entitlement`, `completion`) without
  compatibility aliases. Human-readable output is now the default everywhere;
  `--json` or `--output json` explicitly enables automation output.
- HTTP creation accepts a positional URL, derives its name, and uses the API's
  minimum check interval when omitted. Other defaults and validation remain
  server-owned. Added friendly duration arguments and `--script`.
- Added shell completion, `config paths/show`, and global `--config-dir`.
- Added saved workspace connections with `workspace connect/list/use/disconnect`,
  OS credential storage with protected TOML file fallback, alias selection,
  and environment-token support for CI. Both metadata and file credentials
  use TOML; `--credential-store auto|keyring|file` controls new connections.
- Corrected the hosted API default to `https://primary.fomkee.dev`.
- `--workspace` now accepts a saved alias; `FOMKEE_PROFILE` replaces
  `FOMKEE_WORKSPACE_ID`. Workspace IDs are discovered from API tokens.
- Initial public-beta CLI and monitor-management skill.
- Added the companion check-function authoring skill for Function monitors and
  Heartbeat body validation.
