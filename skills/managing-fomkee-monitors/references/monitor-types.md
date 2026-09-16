# Monitor types

Creation JSON uses `config_type: "http"`, `"function"`, or `"heartbeat"`.
HTTP and Function require the complete active-monitor fields, including URL and
`interval_secs`; Function also requires `js_source`. Heartbeat requires a
flattened schedule fields (`schedule_type: "interval"` with `period_secs`, or
`schedule_type: "cron"` with `cron_expression`) and can include `js_source`
body validation.
Use the public API documentation for the complete current schema.

For simple creation, `fomkeecli monitor create http URL --json` derives a name
from the hostname/path and reads the workspace minimum interval from
`plan.revision.entitlements.monitoring.minimum_check_interval_seconds`.
Use `--name` and `--interval 5m` for explicit choices. Function creation adds
`--script FILE`; Heartbeat creation needs `--name` plus `--every 24h` or
`--cron EXPRESSION`. Optional method, timeout, retry, and grace defaults stay
on the API. File-based create and dry-run requests remain complete API payloads;
the CLI does not fill their missing fields or combine them with convenience flags.


## Updates and alert destinations

Use `monitor update ID --name NAME`, `--interval 5m`, `--url URL`, `--timeout 10s`,
`--method METHOD`, `--description TEXT`, or `--tags a,b` for focused edits. An
empty description or tag argument clears that field. Function and Heartbeat
validation source uses `--script FILE`; Heartbeat schedules use `--every`,
`--cron`, and `--grace`. Unspecified fields and credentials are preserved.
Do not concurrently edit the same object: the API uses last-write-wins PUT.

`monitor update ID --file FILE` accepts a complete editable replacement, not a
patch or a raw detail response. Include explicit nullable fields, full retries,
headers/body, and `auth: {"action":"preserve"}` (or `clear` / `set` with
`credentials`). Heartbeats require a nested `schedule` tagged with `type` and
`body_validation` as JavaScript or null. File input bypasses detail fetching.

Destination commands:

```bash
fomkeecli destination list --json
fomkeecli destination get DESTINATION_ID --json
fomkeecli destination create --file destination.json --json
fomkeecli destination update DESTINATION_ID --name 'On-call' --json
fomkeecli destination update DESTINATION_ID --file replacement.json --json
fomkeecli destination test DESTINATION_ID --json
fomkeecli destination assign DESTINATION_ID MONITOR_ID --json
fomkeecli destination monitors DESTINATION_ID --json
fomkeecli monitor destinations MONITOR_ID --json
fomkeecli destination unassign ASSIGNMENT_ID --json
```

Creation requires `name`, `channel_type` and fields for that channel: email
(`email_address`), webhook (`webhook_url`), telegram (`telegram_bot_token`,
`telegram_chat_id`), ntfy (`ntfy_topic_url`, optional `ntfy_priority`), pushover
(`pushover_user_key`, `pushover_app_token`, optional `pushover_priority`), discord
(`discord_webhook_url`), or slack (`slack_webhook_url`). Do not expose credentials
in arguments or summaries. Use protected files or stdin with `--file -`.

Destination replacement requires all editable fields and the existing channel
type. Write-only Telegram/Pushover/Discord/Slack secrets require explicit
`{"action":"preserve"}` or `{"action":"replace","value":"..."}`; ntfy and
Pushover priority must be supplied. `--name` preserves them automatically.
Assignment lists include monitor/destination IDs and the binding's own ID, which
`unassign` requires. Lists follow explicit pagination. Test delivery outcomes
are `accepted`, `failed`, or `skipped`; inspect the outcome even after exit 0.
