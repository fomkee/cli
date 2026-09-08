# Exit codes

Human-readable diagnostics are the default, including when piped. Pass `--json`
or `--output json` for JSON stdout/stderr and the categories described below.
Argument-validation errors also honor explicit JSON output; help, version,
and shell-completion scripts retain their native text format.

| Code | Meaning |
| --- | --- |
| 0 | The command completed. |
| 2 | Invalid input, workspace/TOML configuration failure (including unsafe file permissions), or required destructive confirmation missing. Configuration failures use JSON category `configuration`. |
| 3 | Credentials are missing, the saved credential is missing, or the OS keyring is unavailable. JSON category is `credentials`. |
| 4 | The API rate limited the request. JSON stderr preserves integer `retry_after_secs`; wait at least that long. |
| 5 | A stable API error was returned; JSON stderr includes its HTTP status and code. |
| 6 | A mutation may have reached the server but its outcome is unknown. Do not replay it automatically. |
| 7 | A read transport failure, response-body read failure after a known HTTP status, or output write/flush failure occurred. A successful mutation is not rolled back by a local output failure; inspect remote state before retrying. |
| 8 | The server response was not compatible with the public API contract. |

For mutation response failures, JSON retains the known HTTP `status` and uses
`mutation_result_unavailable` after a successful status or
`mutation_error_unavailable` after an error status. These retain category
`transport` / exit 7 for unreadable bodies and `protocol` / exit 8 for invalid
JSON or response shapes. Inspect current state before retrying; the CLI never
automatically replays a mutation. Read failures have no mutation recovery code.
Malformed API error envelopes now follow the same protocol-failure rule (8)
rather than masquerading as a stable API error (previously 5 with the synthetic
`invalid_error_response` code); their known HTTP status is still preserved.

Help, version, completion, and ordinary output all return 7 if stdout cannot
be written or flushed, including a broken pipe. Reporting an existing error to
failed stderr is best-effort; the original nonzero exit code is retained.

`skill export` uses code 2 / JSON category `validation` when its destination
already exists. Filesystem failures use code 7 / category `filesystem`. A write
failure can leave a partial export; the error says so. It never merges with an
existing destination or removes partial files automatically.
