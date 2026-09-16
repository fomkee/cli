# Monitor and alert destination management

Select a saved workspace with `--workspace ALIAS`. Add `--json` for the complete
API response in automation. Lists return one page; use `--limit` and the returned
`next_cursor` with `--after` to continue.

## Update a monitor

```bash
fomkeecli monitor update MONITOR_ID --name 'API health' --interval 5m
fomkeecli monitor update MONITOR_ID --url https://example.com/health --timeout 10s
fomkeecli monitor update MONITOR_ID --description '' --tags ''
fomkeecli monitor update MONITOR_ID --script check.js
fomkeecli monitor update MONITOR_ID --every 1h --grace 5m
fomkeecli monitor update MONITOR_ID --cron '0 * * * *'
```

Flags fetch the current detail, change only specified settings, then send the
API's complete replacement. They preserve credentials with `auth.action=preserve`,
as well as existing headers, bodies, scripts, retries, and SLA settings. An
incomplete detail response prevents the write. `--tags` replaces the tag list;
an empty description or tag list clears it. Script files apply to Function
checks or Heartbeat body validation. Duration flags accept values such as `30s`,
`5m`, and `1h`. HTTP-only and Heartbeat-only flags cannot be mixed.

For settings without convenience flags, use a complete replacement file:

```bash
fomkeecli monitor update MONITOR_ID --file replacement.json
```

`--file -` reads stdin. File input is validated for wire shape and forwarded
unchanged, without fetching the monitor or inserting defaults. The editable
schema differs from `monitor get`: omit response metadata, use an explicit auth
action (`preserve`, `clear`, or `set` with `credentials`), and provide every
editable field, including nullable values. Heartbeats use nested
`"schedule":{"type":"interval","period_secs":3600,"grace_secs":60}` or
`{"type":"cron","cron_expression":"0 * * * *","grace_secs":60}`;
`body_validation` is a JavaScript string or `null`.

Updates use the existing last-write-wins PUT API. Avoid simultaneous edits of
the same monitor: another writer can change it between the CLI's read and write.
The CLI never automatically retries a mutation with an uncertain outcome.

## Create and update alert destinations

```bash
fomkeecli destination list
fomkeecli destination get DESTINATION_ID --details
fomkeecli destination create --file destination.json
fomkeecli destination update DESTINATION_ID --name 'On-call'
fomkeecli destination update DESTINATION_ID --file replacement.json
fomkeecli destination test DESTINATION_ID
```

Creation JSON includes `name`, `channel_type`, and channel-specific fields:

| Channel | Required fields | Optional fields |
| --- | --- | --- |
| `email` | `email_address` | |
| `webhook` | `webhook_url` | |
| `telegram` | `telegram_bot_token`, `telegram_chat_id` | |
| `ntfy` | `ntfy_topic_url` | `ntfy_priority` |
| `pushover` | `pushover_user_key`, `pushover_app_token` | `pushover_priority` |
| `discord` | `discord_webhook_url` | |
| `slack` | `slack_webhook_url` | |

For example, save this as `destination.json`:

```json
{"name":"On-call email","channel_type":"email","email_address":"ops@example.com"}
```

Keep secret-bearing JSON in protected files or supply it through stdin;
credentials have no command-line flags. Human output hides credential values and
webhook/topic URLs. Explicit JSON retains the full API response.

`--name` preserves the channel settings and write-only credentials. A complete
update file requires the same channel type; it cannot change a destination's
channel. Telegram tokens, Pushover keys/tokens, and Discord/Slack webhook URLs
use `{"action":"preserve"}` or `{"action":"replace","value":"..."}`.
For ntfy/Pushover replacements, priority is required. Other non-secret settings
retain the creation field names. The API owns channel validation and plan access.

Testing sends a real notification. The response reports `accepted`, `failed`,
or `skipped`. Accepted means the provider accepted it, not that a human read it.
The command exit code reports API/transport success; automation must inspect
`outcome` to distinguish delivery results.

## Monitor assignments

```bash
fomkeecli destination assign DESTINATION_ID MONITOR_ID
fomkeecli monitor assign MONITOR_ID DESTINATION_ID
fomkeecli destination monitors DESTINATION_ID
fomkeecli monitor destinations MONITOR_ID
fomkeecli destination unassign ASSIGNMENT_ID
```

Both listing directions return assignment records with monitor, destination,
and assignment IDs. Remove a binding using its assignment ID. Removing a
binding leaves the monitor and destination available, but stops notifications
through that binding. Destination-side listing requires a server version with
`GET /api/workspaces/{id}/alert-targets/{tid}/alert-assignments`.
