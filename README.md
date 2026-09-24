# Fomkee CLI

Manage your [Fomkee](https://fomkee.com) monitors without leaving your terminal.

Add a health endpoint, keep an eye on a scheduled job, or bring your coding
agent along to help write a custom check. `fomkeecli` connects these workflows
to your Fomkee workspace.

## Getting started

On Linux (x86_64) or macOS, download the latest binary and install it:

```sh
curl -fL "https://github.com/fomkee/cli/releases/latest/download/fomkeecli-$(uname -s)-$(uname -m)" -o fomkeecli &&
  sudo install -d /usr/local/bin &&
  sudo install -m 755 fomkeecli /usr/local/bin/fomkeecli
```

On Windows, download `fomkeecli-Windows-x86_64.exe` from
[Releases](https://github.com/fomkee/cli/releases/latest), rename it to
`fomkeecli.exe`, and put it in a folder on your `PATH`.

Next, open [the Fomkee app](https://app.fomkee.com) and create an API key in your
workspace's settings. Have the token ready, then connect and add your first monitor:

```sh
fomkeecli workspace connect personal
fomkeecli monitor create http https://example.com/health
```

The connection command asks for your token in a hidden terminal prompt.
`personal` is a local name you choose for the workspace.

## A few things to try

See your monitors, then take a closer look at one using its ID:

```sh
fomkeecli monitor list
fomkeecli monitor get MONITOR_ID --details
```

For shell setup, start with `fomkeecli completion --help`.

## Working across workspaces

Keep your own projects and work projects connected:

```sh
fomkeecli workspace connect company
fomkeecli workspace use company
fomkeecli --workspace personal monitor list
```

Each workspace needs its own API key. `workspace use` sets your default;
`--workspace` selects a connection for just that command.

## Bring your coding agent

The CLI includes skills for managing monitors and writing JavaScript checks.
Export them to a new directory:

```sh
fomkeecli skill export ./fomkee-skills
```

Review the exported files, then import the complete skill folders through your
agent's native skill setup. The export doesn't configure your agent for you.

Connect your workspace wherever the agent runs the CLI, keeping the token in
the terminal prompt—not in chat. Then try:

> Monitor https://example.com/health in my personal Fomkee workspace.

## Need a hand?

Start with `fomkeecli --help`, or explore the
[monitor guide](skills/managing-fomkee-monitors/SKILL.md) and
[connection guide](skills/managing-fomkee-monitors/references/authentication.md).

Found a rough edge or have an idea? Open an issue. If you'd like to help with
the code, see [Contributing](CONTRIBUTING.md).

## Update monitors and manage alerts

```bash
fomkeecli monitor update MONITOR_ID --name 'API health' --interval 5m
fomkeecli destination create --file destination.json
fomkeecli destination list
fomkeecli destination assign DESTINATION_ID MONITOR_ID
fomkeecli destination test DESTINATION_ID
fomkeecli destination monitors DESTINATION_ID
```

See the [management guide](docs/MANAGEMENT.md) for destination JSON examples,
updates, credential preservation, pagination, and removing assignments.

## Keep fomkeecli up to date

```bash
fomkeecli self-update --check
fomkeecli self-update
```

GitHub Releases in `fomkee/cli` is the source for stable updates. Interactive
API commands check at most once daily using a local cache and print an available
version to stderr. Checks have a two-second budget and failures do not affect
your command. JSON, redirected output, help, completion, configuration inspection,
and skill export do not trigger automatic checks.

Disable notifications with `--no-update-check` or `FOMKEE_NO_UPDATE_CHECK=1`.
`self-update --check` always performs an explicit check; neither update command
needs workspace credentials. Nothing is installed automatically.

Self-update downloads the matching Linux x86_64, macOS arm64/Intel, or Windows
x86_64 release binary, verifies its size and SHA-256 checksum, then replaces the
executable while preserving permissions. It never downgrades, installs a
prerelease, or falls back to an unchecked download. The installation directory
must be writable; there is no automatic privilege escalation. The next invocation
uses the new version. Other platforms must build from source.

## Incidents and maintenance

```sh
fomkeecli incident list
fomkeecli incident list --monitor MONITOR_ID
fomkeecli incident get INCIDENT_ID
fomkeecli incident timeline INCIDENT_ID
fomkeecli incident post INCIDENT_ID --message "We are investigating the outage."
fomkeecli incident post INCIDENT_ID --message-file update.txt

fomkeecli maintenance list
fomkeecli maintenance create --title "Database upgrade" \
  --start 2030-10-01T09:00:00+02:00 --end 2030-10-01T10:00:00+02:00 \
  --monitor MONITOR_ID
fomkeecli maintenance cancel MAINTENANCE_ID
```

Replace example dates and IDs with your intended schedule and monitors. Notes
are public-facing, immutable updates visible on status pages containing the
monitor; they do not change incident status. Maintenance announces planned work;
**checks, incidents, and alerts continue normally**.

Use `--json` for automation and `--after` for subsequent pages. Incident listing
includes all states; `open` and `regressed` identify ongoing incidents. Maintenance
creation requires timezone-explicit start/end times and one or more `--monitor`
IDs. It also accepts a complete JSON file through `--file` (`-` reads stdin).
See the bundled [agent workflow](skills/managing-fomkee-monitors/references/incidents-maintenance.md)
for examples, input shape, and handling uncertain outcomes.
