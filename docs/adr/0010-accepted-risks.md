# ADR-0010: Accepted risks — unbounded HTTP sessions, the harness reader thread, the cost of a tool

Status: accepted (2026-10-27)

## Context

The hardening pass that produced the `Secret` type and bounded response reads left three findings
unfixed. Each is recorded here with its reason so a future audit meets a decision rather than
rediscovering the finding and re-proposing a fix that cannot land.

## Decision

Three risks are accepted, not fixed.

**The HTTP session store has no cap on session count.** Every `initialize` creates a session entry
that lives until it idles out. `rmcp` 3.7.0 exposes no limit on that map, so capping it means
patching or wrapping a dependency we do not own. The exposure is small: idle sessions close after
300 s, so the steady-state count is bounded by how many clients reconnect within that window rather
than by total connections ever made, and reaching the transport at all requires the bearer token.
An unauthenticated peer cannot grow the map. Whoever can grow it already holds the key to the
server. Revisit only if `rmcp` grows a session cap, or if the bind address ever leaves loopback.

**The harness's stdout reader thread can outlive its client.** The `query_reports` harness spawns
the server and drains its stdout on a detached thread. If the client side goes away first, that
thread blocks on a pipe nobody reads and is not joined. Fixed properly it needs either a second
shutdown channel through every call site or a reader that can be cancelled mid-read. Neither
belongs in a pass whose whole point was bounded reads. The harness is a development binary: it
ships in no release, serves no user, and dies with the test process. The leak is real, small, and
confined to `cargo run` on a developer's machine. Tracked as backlog, not as a defect.

**Adding a tool costs eight touch points across five files.** The registry entry, the argument
builder, the schema helpers, the per-tool module, the dispatch arm, and the harness/test surface
all change together. That is the deliberate shape of ADR-0007's closed enum: the compile errors at
each site are the reason a tool can never be half-registered or silently unroutable. It is not
shallowness that a table or a macro would remove — a `struct ToolSpec { build: fn }` table only
moves the match inside and loses the exhaustiveness check that catches a forgotten arm. The cost is
paid once per tool, by someone who is already reading all five files, and it buys a class of bug
that cannot exist. No redesign here.

## Consequences

- Every one of these is a known, bounded, and written-down cost rather than an oversight. A future
  audit that raises one of them is pointed here.
- The two that could reach a user are gated by something stronger than the risk itself: the bearer
  token for sessions, the dev-only binary for the reader thread.
- ADR-0007 owns the tool-registry cost. Changing it means superseding that ADR, not adding a fix
  here.
