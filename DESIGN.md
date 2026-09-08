# fomkeecli design rules

This is the evolving, canonical UX specification for the CLI. Human contributors
and agents use the same rules. Intentional design changes must update this
document, representative examples, and output regression tests together.

## Command language and API boundary

- The executable is `fomkeecli`. Use singular nouns and consistent verbs.
- Common operations need one invocation, not a wizard. Keep
  `monitor create http|function|heartbeat` and shell completion discoverable.
- The API owns validation, entitlements, defaults, and lifecycle policy.
  Presentation formats the response; it does not infer health or fetch extra
  data. In particular, an active monitor is not necessarily healthy.
- Pagination is explicit. Show the returned cursor; never fetch another page
  simply to fill a screen. Saved workspace completion is currently a snapshot.
- Validate required wire fields at the HTTP boundary with CLI-owned DTOs.
  Carry typed command results into presentation; malformed required fields are
  protocol errors, not successful pages filled with `Unknown`. Keep extensible
  API sections flexible without duplicating server business rules.
- Retain original response JSON alongside the typed view so machine output
  preserves extension fields and missing-versus-null distinctions. Complete
  file requests are likewise validated for shape and forwarded unchanged;
  convenience defaults must never override them.

## Output contract

- Human-readable output is the default, including pipes and files. Only
  `--json` or `--output json` selects machine output. Never auto-select JSON.
- JSON keeps the complete existing response structure, no ANSI or prose, and
  never prompts. Presentation flags do not change JSON data or exit status.
- Results go to stdout; errors and interactive prompts go to stderr. Completion
  scripts remain native shell text, with no headings or styling.
- Failed stdout writes or flushes, including help, version, and completion,
  exit with code 7. Broken pipes follow that same policy, not success. Reporting
  an existing failure to stderr is best-effort if stderr itself has failed;
  this must never turn the failing command into a successful exit.
- `--details` includes secondary configuration and audit metadata. It is not a
  debug-log switch, a secret-reveal flag, or an additional API request.
- Keep ordinary output in terminal scrollback. No alternate screen, automatic
  pager, animation, decorative banner, or cursor movement.

## Visual language

| Role | Style | Meaning |
| --- | --- | --- |
| Title | Bold default foreground | Collection heading or action outcome |
| Section | Bold cyan | Information grouping |
| Success | Green with explicit text | Confirmed successful operation or passed check |
| Warning | Yellow with explicit text | Actionable degradation or unavailable execution |
| Failure | Red with explicit text | Error or failed check |
| Hint | Dim default foreground | Optional next action |
| Secondary reference | Dim default foreground | Monitor name and workspace ID next to its name |
| Active lifecycle | Green dot with explicit Active label | Monitoring enabled, not a health verdict |
| Data | Default foreground | Names, URLs, IDs, values; readable on light/dark themes |

Do not rely on color alone, use emoji as status, color entire rows, or equate
paused/disabled/unknown lifecycle states with failures. Avoid hard-coded RGB
backgrounds and heavy boxes. Labels use normal foreground, not faint gray.

`--color auto|always|never` applies independently to each output stream. Auto
styles terminals only and honors nonempty `NO_COLOR` and `TERM=dumb`. Explicit
always/never wins over environment preferences; JSON is always plain. Help and
argument errors follow the same color policy.

## Layout by command family

- **Lists:** title and page count, compact whitespace-separated table, then an
  explicit continuation hint when a cursor exists. On narrow terminals switch
  to labeled records. Names and IDs stay identifiable; IDs are never truncated.
- **Monitor get:** start with `● Active · HTTP` (or the API's actual lifecycle
  and type), a blank line, then the target. Add a blank line before and after
  the page and indent its content consistently. Names belong in References,
  not a bold heading. Details adds meaningful request settings, retries,
  function/schedule configuration, and spaced History records.
- **References:** show the monitor name subdued and its ID copyable. Show a
  workspace name from the already-selected saved profile, with its ID dimmed
  alongside it. If a name is unavailable, omit the workspace reference rather
  than printing an unexplained UUID. Never substitute an alias for a real name
  or make a decorative lookup. Omit creator UUIDs; the current API supplies no
  creator display name.
- **In Fomkee:** link to `https://app.fomkee.dev/monitors/{id}` in monitor get
  and successful create/lifecycle output for the hosted primary API only.
  Validate the monitor ID before constructing the URL. Keep it plain and
  copyable; do not automatically open a browser or fabricate links for custom
  API origins. The browser must already have the correct workspace selected.
- **Mutations:** action and target first, copyable ID, relevant state/target.
  Do not print the complete object unless details was requested.
- **Auth/config/workspaces:** clearly separate selected connection, credential
  source, and file locations. Config inspection must not read token values.
  A local workspace disconnect does not revoke the server key: say so.
  Supported file fallback is normal behavior, not a warning. Show the backend
  neutrally; explain encryption and owner-only permissions in documentation.
  Only warn about an actual problem with a useful recovery action.
- **Entitlements:** plan identity then limits grouped by API capability section.
  Never interpret null as unlimited without an API contract saying so.
- **Skill export:** `skill export DIRECTORY` writes complete, version-matched
  skill folders for review and native harness import. Require a new destination
  with an existing parent; never merge or overwrite, including empty folders
  and destination symlinks. Show the absolute destination, CLI version, skill
  names, and a short review/import hint. No harness detection or configuration
  edits, automatic installation, network requests, or credential/config reads.
  JSON returns the destination, version, and per-skill file manifest. Report
  partial write failures without deleting potentially user-modified files.
- **Dry-run:** overall verdict first, then HTTP/function results and evidence.
  Details adds attempts and execution metadata. Non-execution is not success.
- **Errors:** what failed, API status/code where available, relevant details,
  then a concrete recovery hint where known. Preserve retry-after and ambiguous
  mutation warnings; never advise blindly retrying an unknown mutation outcome.
  A successful HTTP status with an unreadable mutation result retains that
  status and uses `mutation_result_unavailable`; inspect current state first.
  An unreadable mutation error response uses `mutation_error_unavailable` and
  also requires inspection. Body-read and decode failures retain exit codes
  7 and 8 respectively; exit 6 is reserved for unknown transport outcomes
  before a status is available. Never automatically replay a mutation.
- **Empty results:** explain what is empty and give a useful next command.

## Data formatting and safety

- Order by importance, not alphabetical field name. Nest into named sections;
  use a bounded generic renderer only for extensible API sections.
- Display readable enums, exact friendly durations (`5 minutes`, `1 minute
  30 seconds`), and timestamps such as `7 Sep 2026 at 21:05:58 UTC`. Human
  History omits fractional seconds; JSON retains full precision and original
  units. Separate Created and Updated records with whitespace.
- Omit empty optional description/tags from summaries. Distinguish unknown,
  not configured, and not applicable where the API lets us do so. Do not
  invent defaults for missing fields.
- Omit unconfigured auth, empty headers, absent bodies, and empty sections,
  including under `--details`. More detail means useful information, not every
  possible field. Keep configured sensitive content hidden.
- Wrap prose and table cells to available width without silently losing data.
  Use stacked records below 80 columns. Keep IDs and file paths copyable,
  allowing an unbroken value to exceed very narrow widths. When stdout is not
  a terminal use a deterministic 100-column layout.
- Escape terminal control characters and bidirectional formatting controls in
  all API-provided text before applying our own styles.
- Human detail views summarize potentially sensitive headers, bodies, scripts,
  and auth rather than dumping their content. Generic fields with secret names
  are redacted. Do not claim arbitrary server messages or user data can be
  perfectly secret-scanned. JSON remains the explicit full API result.
- The one-time Heartbeat creation secret is the deliberate exception: display
  it once with a prominent storage warning, including with `--details`.

## Representative detail

```text

  ● Active · HTTP

  GET https://example.com/health

  Checks
    Every     5 minutes
    Timeout   10 seconds
    Expected  Any successful HTTP status

  References
    Name       Rbasovo
    Monitor    361aca90-733d-4b11-a398-ad961bfd7e48

  In Fomkee
    https://app.fomkee.dev/monitors/361aca90-733d-4b11-a398-ad961bfd7e48

  Use --details for secondary settings and history.

```

## Implementation and verification

Use `comfy-table` for table layout, `anstyle` for semantic styles, and `anstream`
for terminal-aware writing. Keep styles, value formatting, layout primitives,
and command-family views separate. Ratatui is not needed for print-and-exit
commands; reconsider it only for an explicitly requested interactive feature.

Use current stable dependencies for new work. Review migration APIs and test
upgrades rather than guessing version constraints. Document concrete exceptions
and platform compatibility in [dependency notes](docs/DEPENDENCIES.md).

Cover representative output with checked-in snapshots and behavioral tests:
40/80/120-column layouts, Unicode names, long values, empty lists, pagination,
color on/off, pipe behavior, details, JSON invariance, malicious terminal text,
and one-time secrets. Tests must be deterministic and offline.
