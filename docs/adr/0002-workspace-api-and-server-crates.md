# ADR-0002: Workspace split — `hackerone-api` + `hackerone-mcp`

Status: accepted (2026-10-27)

## Context

The upstream Python server mixes a stdlib HTTP client (`client.py`) with MCP plumbing
(`server.py`). Rust benefits from a seam that keeps `rmcp` out of the client, and the
dependency graphs differ: the client needs `reqwest`/`serde`; the server needs `rmcp`,
transport crates, `clap`, `toml`.

## Decision

Workspace with two members:

```
crates/hackerone-api/   # typed REST client, serde models, Error enum. No MCP types.
crates/hackerone-mcp/   # tool schemas, dispatch, config, transports.
                        # bins: hackerone-mcp, hackerone-mcp-query-reports-harness
```

- The api crate owns the `Error` enum (one variant per cause) and never panics on API
  responses.
- The server crate maps `Error` onto MCP `isError` text, and owns all JSON-Schema literals.
- No third crate unless a second consumer appears (YAGNI).

## Consequences

- Client logic is testable against a fake HTTP server without spinning an MCP transport.
- Two version numbers to keep in lockstep during 0.x; the server pins the api crate by path.
