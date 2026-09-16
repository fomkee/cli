# Safety

The CLI does not retry mutation timeouts. Exit category `outcome_unknown` means
the server may have performed the action; list/get the affected monitor or destination before deciding
what to do next. Delete needs explicit user confirmation and `--yes`. Do not
log API tokens, custom authentication, request bodies, JavaScript source, or
one-time Heartbeat secrets.
