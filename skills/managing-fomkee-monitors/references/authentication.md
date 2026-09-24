# Authentication

Each workspace connection uses its own existing API key. `workspace connect
ALIAS` validates the key through `/api/session`, fetches the workspace's ID and name,
and saves the token in the OS keyring or protected TOML file. The default API origin is
`https://primary.fomkee.com`; `connect --api-url ORIGIN` can select another
deployment. No browser login or account-wide credential is involved.

Use `fomkeecli config paths --json` to discover resolved file locations and
`config show --json` to inspect effective non-secret connection metadata.
`--config-dir` overrides `FOMKEE_CONFIG_DIR` and the platform default. These
diagnostics do not access tokens or contact the API.

Use `workspace list --output json` for local connections and `--workspace ALIAS`
on every command in a workflow. This selects both the saved key and saved
origin. Selection precedence is `--workspace`, `FOMKEE_PROFILE`, environment
token, then active saved workspace. Unknown aliases fail without fallback.
`workspace use ALIAS` changes the default for future invocations; agents should
prefer the per-command flag to avoid changing another caller's default.

`FOMKEE_API_TOKEN` supports CI and headless machines; `FOMKEE_API_URL` selects its
API origin. These variables do not override an explicitly selected saved
connection. `FOMKEE_WORKSPACE_ID` is no longer supported; unset it, since a key
already identifies one workspace. `workspace list` is local inventory and
cannot discover other account workspaces using a single key.

New connections default to `--credential-store auto`: prefer the OS keyring,
then fall back to owner-only, unencrypted `credentials.toml` if the keyring
write is unavailable. The result reports the actual `credential_store` and a
file-storage notice. Explicit `keyring` disables fallback; explicit `file`
bypasses the keyring. `workspaces.toml` records the backend per connection.
Reads and disconnects never switch backends. For an existing locked keyring
entry, ask the user to unlock it or use an ephemeral environment connection.
Never read credential-file contents into chat or commit them to a repository.
To connect noninteractively, use `--token-stdin` or the environment;
explicit `--output json` never prompts. `workspace disconnect ALIAS` removes
only the local connection and saved token, not the remote API key. Replacing
a saved token requires disconnecting and reconnecting the alias.

Run `fomkeecli --workspace ALIAS auth status --output json` before a write.
Inspect `connection.api_url` and the resolved workspace ID. A transport error
is not a rejected token; HTTP 401 is. If a server refuses a write with
`primary_only`, do not replay the mutation automatically; correct the
environment origin or reconnect the saved alias to the primary API.

Authenticated requests share a per-workspace bucket on each API node,
regardless of whether authentication uses this API token or a user session.
The fixed UTC-minute plan limits are Indie 60, Starter 180, and Pro 600. On
HTTP 429, require the stable `rate_limited` code and read
`retry_after_secs` from the CLI JSON error; wait at least that many seconds.
The CLI retries only a read whose wait is at most five seconds and never
replays a mutation. Buckets are not cluster-wide: switching nodes can add that
node's allowance, and restarting a node clears its current bucket.
