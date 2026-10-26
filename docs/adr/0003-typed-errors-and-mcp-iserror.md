# ADR-0003: Typed errors in the api crate, MCP `isError` at the tool boundary

Status: accepted (2026-10-27)

## Context

Upstream returns `{"error": {"status", "detail"}}` as **successful** tool text, so MCP clients
cannot distinguish failure from data. It also silently coerces malformed payloads to `{}`/`[]`.

## Decision

- `hackerone_api::Error` is a `thiserror` enum, one variant per cause:
  `Api { status, detail, body }`, `Transport(#[from] reqwest::Error)`,
  `Decode(#[from] serde_json::Error)`, `Validation { field, reason }`,
  `Config { field, reason }`, `ProgramNotFound { handle }`, `MissingProgramHandle`.
- Non-2xx API responses become `Api`, never text.
- The server maps `Error` to a failed tool result (`isError: true`) whose text is
  `<variant>: <detail>`; success results stay raw JSON text.
- No `anyhow` in the api crate. The server may use `anyhow` only in `main`/setup, not in
  tool handlers.

## Consequences

- Agents see real MCP tool failures and can retry/branch on them.
- Parity break for callers that parsed `{"error": ...}` from text — documented in the README
  differences table.
- Every API failure carries a machine-checkable cause through the whole call path.
- Response models decode permissively: fields optional or `#[serde(default)]`, unknown keys
  ignored, no `deny_unknown_fields`. A sparse-but-valid payload must never surface as
  `Decode`. Until typed models land, the client returns `serde_json::Value` (no strict
  coercion, no `{}`/`[]` fallback).
