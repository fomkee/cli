---
name: managing-fomkee-monitors
description: Manage Fomkee HTTP, Function, and Heartbeat monitors through fomkeecli with explicit JSON output. Use for monitor inspection, creation, updates, lifecycle changes, alert destinations and assignments, incident inspection and public notes, and planned maintenance; do not use for invitations or MCP.
---

# Manage Fomkee monitors

Use only the `fomkeecli` command and its public HTTP API contract. Begin by checking
`fomkeecli --version` and `fomkeecli workspace list --output json`. For a saved
connection, use `--workspace ALIAS` consistently on `auth status` and subsequent
commands so another process changing the active default cannot redirect work.
If several connected workspaces could match the request, resolve the target
with the user before writing. Run `auth status --output json` for the selected
connection and check its workspace ID and `connection.api_url`.

If credentials are missing, direct the user to `fomkeecli workspace connect ALIAS`
in their terminal, or to supply `FOMKEE_API_TOKEN` through their environment for
automation. Never ask them to paste a token into chat or a command argument.
Read [authentication](references/authentication.md) for precedence and credential-store
availability before changing a connection.

Inspect `fomkeecli entitlement show --output json` and one page from `fomkeecli
monitor list --output json` before proposing a creation. Follow
`next_cursor` with an explicit `--after` only when the workflow needs more
results. Server validation and effective entitlements are authoritative: do
not invent identifiers, plan limits, or required values.

For monitor creation, obtain a complete request JSON file using the matching public
schema. Dry-run HTTP and Function configurations first using `fomkeecli monitor
dry-run --file FILE --output json`. Heartbeats have no unsaved dry-run API.
When a Function monitor or Heartbeat validator needs `js_source`, use the
companion `$writing-fomkee-check-functions` skill to author or review the
script; keep this skill focused on monitor lifecycle and mutation safety.
Create with `fomkeecli monitor create http|function|heartbeat --file FILE
--output json`, then report the returned monitor ID and next safe action.

Always pass `--json` or `--output json` in automation: human output is the default,
even when piped. Command nouns are singular (`monitor`, `entitlement`); no plural
aliases exist. Treat stderr JSON as a structured error: report
validation/API failures accurately; for `outcome_unknown`, do not retry a
mutation—inspect current state first. A Heartbeat create may return a one-time
secret. Tell the user to store it securely, but do not repeat it in summaries.

Pause, resume, disable, and enable require a real monitor ID. Before deleting,
show the exact target and ask the user for explicit confirmation. Only then run
`fomkeecli monitor delete ID --yes --output json`; never infer consent from a
general request to clean up.

Read [authentication](references/authentication.md) for credential handling,
[monitor types](references/monitor-types.md) for required input shape, and
[safety](references/safety.md) for destructive and uncertain outcomes.


For monitor updates and destination management, read the
[update and destination guidance](references/monitor-types.md#updates-and-alert-destinations).
Use `monitor update ID` flags for focused edits; complete replacement JSON needs
explicit credential actions. Destination tests send real notifications, so run
them when the user's task calls for testing delivery. Assignment removal changes
alert routing; identify the exact binding before removing it.

`fomkeecli self-update --check --json` checks GitHub's latest stable version without
installing. Run `self-update --json` when the user requests updating the CLI;
monitor-management authorization alone does not request executable replacement.
JSON commands never trigger automatic version discovery.

For incident inspection, public notes, or scheduling/cancelling maintenance, read
[incidents and maintenance](references/incidents-maintenance.md). Maintenance
announces planned work; checks and alerts continue. Incident posts are public
updates, not private comments, and do not change incident status.
