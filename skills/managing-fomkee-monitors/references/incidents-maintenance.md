# Incidents and planned maintenance

Use the selected saved workspace consistently and request JSON output. These
commands use existing incident and maintenance APIs; the server enforces
permissions and scheduling rules.

## Inspect incidents and publish updates

```sh
fomkeecli --workspace production incident list --json
fomkeecli --workspace production incident list --monitor MONITOR_ID --json
fomkeecli --workspace production incident get INCIDENT_ID --json
fomkeecli --workspace production incident timeline INCIDENT_ID --json
fomkeecli --workspace production incident post INCIDENT_ID --message-file note.txt --json
```

`incident list` includes open, regressed, and resolved incidents. For an ongoing
incident, inspect `state` (`open` or `regressed`), monitor identity, and timeline
before selecting it. Follow `next_cursor` explicitly when more results are needed;
do not assume the first page contains every ongoing incident.

`incident post` publishes a public-facing status update. Notes appear on status
pages that include the monitor; this is not a private/internal comment. Notes
are immutable and may also be posted after resolution. Posting does not resolve,
reopen, or otherwise change an incident's status.

Publish when the user's task requests publication. A request to draft a note
only calls for a draft. Use facts supplied by the user or confirmed by the
incident; do not invent a cause, recovery, or estimated fix time. Exclude secrets
and internal-only details. If the incident or intended message is ambiguous,
resolve that ambiguity before writing. Do not ask again when the user has
already specified the target and authorized the publication.

Provide 1–2000 characters using `--message TEXT`, or `--message-file FILE` for
multiline text (`-` reads stdin). On success, report the returned event ID and
message. If the mutation outcome is unknown or its result unreadable, inspect
the incident timeline for the note before considering another attempt; never
blindly repost and create duplicates.

## Schedule maintenance

```sh
fomkeecli --workspace production maintenance list --json
fomkeecli --workspace production maintenance create \
  --title "Database upgrade" \
  --description "The API may be briefly unavailable." \
  --start 2030-10-01T09:00:00+02:00 \
  --end 2030-10-01T10:00:00+02:00 \
  --monitor MONITOR_ID --json
fomkeecli --workspace production maintenance cancel MAINTENANCE_ID --json
```

The example dates are placeholders. Resolve dates such as “tomorrow night” using
the user's intended timezone; ask if it is unknown. Both timestamps require an
explicit UTC offset or `Z`. Resolve the named monitors from `monitor list` and
inspect existing maintenance for duplicates before scheduling. Repeat `--monitor`
or supply comma-separated IDs to select multiple monitors.

Maintenance is a public announcement of planned work. Checks, incidents, and
alerts continue normally. Never use maintenance to mute alerts or conceal an
ongoing outage. The server requires a future start, an end after the start, and
at least one affected monitor. It determines lifecycle and availability/SLA
behavior; do not fabricate a successful schedule when validation fails.

Scheduling is appropriate when the user asks to schedule it and the dates,
monitors, and public wording are clear. A request for a proposed plan alone does
not authorize creation. Report the returned maintenance ID, timezone-explicit
start/end, and affected monitors. Cancellation uses the ID from
`maintenance list`; editing a window is not exposed by the current API.

For complete JSON input, use `maintenance create --file FILE` (`-` reads stdin):

```json
{
  "title": "Database upgrade",
  "description": "The API may be briefly unavailable.",
  "scheduled_start": "2030-10-01T09:00:00+02:00",
  "scheduled_end": "2030-10-01T10:00:00+02:00",
  "monitors": ["00000000-0000-0000-0000-000000000002"]
}
```

Replace example IDs with real workspace monitor IDs. File input is complete and
cannot be mixed with individual settings. On an uncertain creation/cancellation
outcome, inspect maintenance pages before retrying.
