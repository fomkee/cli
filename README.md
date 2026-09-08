# Fomkee CLI

Manage your [Fomkee](https://fomkee.dev) monitors without leaving your terminal.

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

Next, open [the Fomkee app](https://app.fomkee.dev) and create an API key in your
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
