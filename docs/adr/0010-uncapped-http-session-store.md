# ADR-0010: The HTTP session store is uncapped

Status: accepted (2026-10-27)

## Context

Hardening the transport raised one risk that this codebase cannot close from its own source: every
MCP `initialize` creates a session entry in `rmcp`'s `LocalSessionManager`, and `rmcp` 3.7.0 exposes
no limit on how many entries the map may hold.

## Decision

The cap stays unset. We accept an uncapped session map rather than wrap or patch a dependency we do
not own.

The exposure is bounded twice over, and neither bound is ours to enforce: idle sessions close after
300 s, so the steady-state count tracks reconnects inside that window rather than connections ever
made; and reaching the transport at all requires the bearer token, so an unauthenticated peer cannot
grow the map, and whoever can grow it already holds write access to the HackerOne program.

## Consequences

- A bearer-token holder can still exhaust memory by opening sessions faster than they idle out. That
  is a self-inflicted denial of service by a principal who can already act on every tool.
- Revisit if `rmcp` grows a session cap — adopt it, delete this ADR — or if the bind address ever
  leaves loopback, where the bearer token stops being a meaningful gate.
