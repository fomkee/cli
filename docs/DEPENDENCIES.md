# Dependency policy and migration notes

Use current stable releases for new work. Check upstream documentation and
review behavior changes before upgrading. Record a concrete reason whenever
a dependency must stay behind; do not force transitive overrides just to remove
Cargo's newer-version notice. The lockfile remains the reproducible build input.

## Reviewed update — 2026-09-08

- **comfy-table 8.0.0:** migrate `load_preset` to `load_style`. Preserve whitespace
  tables, ID column constraints, and Unicode wrapping through output tests.
  [Upstream changelog](https://github.com/Nukesor/comfy-table/blob/main/CHANGELOG.md)
- **reqwest 0.13.4:** select `json` and `rustls` without default features. This
  uses the current Rustls/AWS-LC and platform certificate-verifier stack.
  Retain redirect refusal, timeouts, authentication, response contracts, and
  mutation retry policy. [Features](https://docs.rs/crate/reqwest/0.13.4/features)
- **toml 1.1.5:** parse documents with `toml::from_str`, including test fixtures;
  `Value::from_str` now parses a value rather than a whole document. Existing
  profile and credential TOML remains readable, including UUID table keys.
  [Documentation](https://docs.rs/toml/latest/toml/)
- **Keyring's current ecosystem:** use `keyring-core 1.0.0` and explicit
  `dbus-secret-service-keyring-store 1.0.1` (Linux),
  `apple-native-keyring-store 1.0.2` (macOS Keychain), and
  `windows-native-keyring-store 1.1.0` (Credential Manager).
  The package now called `keyring 4.x` is a wrapper; the upstream-recommended
  modular API lives in these packages. Avoid installing a global default store
  or pulling in unnecessary backends. [Upstream explanation](https://docs.rs/crate/keyring/4.2.0)

## Saved OS credentials

Keep the existing service `fomkeecli` and the profile's credential UUID as the
user specifier. Never generate new IDs merely because a library was upgraded.
The selected stores preserve these native lookup conventions:

- Linux: `service` and `username` attributes; find existing entries across
  collections without requiring new attributes.
- macOS: User/login Keychain, the same service and account attributes.
- Windows: `{credential_uuid}.fomkeecli` target name and UTF-16 password encoding.

These mappings were reviewed against the old and new provider source. Offline
tests assert the service/user specifiers and file-store round trips. Native
read/write interoperability with an existing keyring must still be verified on
each release platform; passing an in-memory test is not that verification.
Protected TOML fallback, backend selection, and token secrecy remain unchanged.

## Current transitive exception

`generic-array 0.14.7` remains because the Linux Secret Service cryptography
chain uses `crypto-common 0.1.7`, which pins **exactly `=0.14.7`**. Version
0.14.9 cannot satisfy that requirement. The current platform-store upgrade does
not remove this upstream constraint. Revisit when that dependency chain updates;
do not add a direct dependency or patch cryptography code to override it.

Inspect the source with `cargo tree -i generic-array@0.14.7` from this workspace.
