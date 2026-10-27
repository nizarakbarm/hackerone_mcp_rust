# ADR-0009: Bounded requests, and the rmcp shutdown-drain limit

Status: accepted (2026-10-27)

## Context

An audit found two agent-facing hazards:

1. `hackerone-api::Client` built `reqwest::Client` with **no timeout**. A HackerOne request that
   never completes blocks the MCP tool call forever; the agent has no recovery path.
2. `rmcp` 3.7/3.8 hardcodes a **5 s drain window** on `QuitReason::Closed` (stdin EOF) and
   `2 s` on `Cancelled` (`rmcp/src/service.rs`). A tool still running when the client closes stdin
   is cancelled and its result is never delivered — observed live as
   `WARN rmcp::service: timed out draining in-flight responses`.

rmcp exposes no knob for that drain window in either version (verified in the 3.8.0 source).

## Decision

- **Timeouts are constant, in the api crate**: `connect_timeout = 10 s`, per-request `timeout =
  30 s` on the `reqwest::Client`. Per-request, not per-tool-call: `search_programs` legitimately
  issues up to 20 sequential requests, so a single total budget would break a shipped tool.
  No config field until an operator reports HackerOne latency above 30 s.
- **The drain window is a documented limitation, not fought.** We do not vendor or fork rmcp and do
  not hand-roll a stdio loop to win 5 s. Instead:
  - the per-request timeout bounds how long any tool can keep a connection busy;
  - a tool that outlives the drain window exits cleanly, with a log line — never with a panicked
    task or a truncated JSON-RPC frame;
  - the README states that the MCP client must keep stdin open until the response arrives, which is
    what real hosts do.
- **The client validates its own paths**: `Client::request` rejects paths that do not start with
  `/` or that contain a `..` segment, so the seam cannot be abused by a future caller that forgets
  to validate an id or handle.

## Consequences

- A stalled HackerOne endpoint now yields `Error::Transport` after 30 s instead of hanging forever.
- Long scans remain possible (20 requests × 30 s worst case), but a client that closes stdin early
  still loses that call's result. Accepted: fixing it needs an rmcp API that does not exist.
- Path validation turns a hypothetical injection into an immediate typed `Validation` error.
