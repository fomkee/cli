---
name: writing-fomkee-check-functions
description: Write, review, or debug JavaScript checks for Fomkee Function monitors and Heartbeat body validation. Use for the injected response global, throw-on-failure behavior, dry-run console output, sandbox capabilities, or entitlement-dependent runtime limits; do not use for monitor lifecycle operations or Fomkee's internal compiler and executor.
---

# Write Fomkee check functions

Write a top-level JavaScript script body. Do not export or define an uncalled
check function.

- Reaching the end without throwing passes the check.
- Throwing an `Error` fails it; the message becomes the failure reason.
- Returned values and bare boolean expressions are ignored.
- Failure messages should identify the expected and observed values.

```js
if (response.status !== 200) {
  throw new Error("expected status 200, got " + response.status);
}
```

Fomkee injects this global:

```js
var response = {
  status: Number,
  body: String,
  headers: Object,
  latency_ms: Number,
};
```

Parse JSON bodies explicitly with `JSON.parse(response.body)`. Header names are
lowercase, for example `response.headers["content-type"]`. Function monitors
receive the final HTTP response after retries. Heartbeat validators receive the
submitted request body and headers with synthesized `status: 200` and
`latency_ms: 0`. Bodies are strings, not byte arrays.

## Diagnose with dry-run output

`console.log`, `console.info`, `console.warn`, and `console.error` are available.
Dry runs capture bounded console output, while scheduled checks do not persist
it. Logging never fails a check, including `console.error`; throw an `Error`
when validation must fail.

```js
const data = JSON.parse(response.body);
console.log("parsed status", data.status);

if (data.status !== "ok") {
  throw new Error("expected body status ok, got " + data.status);
}
```

For a Function configuration, place the complete monitor request in a file and
run `fomkeecli monitor dry-run --file FILE --output json` before creation. Test a
passing response and every important failure case. Fomkee has no unsaved
Heartbeat-validator dry-run endpoint; do not claim that the CLI can dry-run one.

## Respect the sandbox and current limits

Use standard ECMAScript facilities such as `JSON`, `Date`, `Math`, strings,
arrays, maps, sets, regular expressions, typed arrays, `TextEncoder`, and
`TextDecoder`.

Do not use:

- `fetch`, `XMLHttpRequest`, or other network access;
- `require`, `import`, Node built-ins, or package dependencies;
- `process`, `window`, or filesystem APIs.

Execution receives wall-clock, fuel, memory, and maximum-input-body budgets
from the workspace's effective plan revision. Retrieve current values with
`fomkeecli entitlement show --output json` before quoting limits; never hard-code
plan values.

## Author and review checks

1. Identify whether the script validates a Function monitor response or a
   Heartbeat request body.
2. Validate only load-bearing properties instead of mirroring an entire schema.
3. Parse and validate types before using nested values.
4. Throw one actionable error at the first failed invariant.
5. Use console output only for temporary dry-run context.
6. Exercise a passing response and each important failure path where the
   available monitor workflow permits it.

Reject uncalled `function check(response) { ... }` wrappers, `return false`,
axios-style `response.data`, title-cased header lookups, `console.error` without
a throw, unavailable platform APIs, and runtime-limit claims not read from the
current entitlements.
