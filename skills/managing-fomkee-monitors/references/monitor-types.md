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
