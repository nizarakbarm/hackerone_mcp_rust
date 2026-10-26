# ADR-0005: Dual transport, bearer-gated streamable HTTP

Status: accepted (2026-10-27)

## Context

Upstream is stdio-only. Remote/container use needs the MCP streamable-HTTP transport, but an
unguarded HTTP MCP endpoint exposes a write-capable HackerOne client (state changes,
comments) to anyone who can reach the port.

## Decision

- Both transports compiled in by default; selected at runtime with `--transport stdio|http`.
- HTTP defaults: bind `127.0.0.1:8765`, overridable with `--bind` / `[http] bind`.
- **Every** HTTP request must carry `Authorization: Bearer <HACKERONE_MCP_HTTP_TOKEN>`.
  Missing/incorrect token → HTTP 401, no MCP session created. No loopback exemption.
- No in-process TLS: terminate at a reverse proxy. `--bind` to a non-loopback address prints a
  warning at startup.
- Credentials for the upstream HackerOne API come from the layered config (ADR-0004), not from
  the incoming HTTP Authorization header. Single-tenant process.
- stdio mode has no auth layer and never logs to stdout.

## Consequences

- Deploying remotely requires setting one extra env var; forgetting it fails closed.
- Multi-tenant / per-request-credential use is out of scope until a new ADR says otherwise.
