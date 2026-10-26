# ADR-0007: Tool registry shape — closed enum, per-field schema helpers, router-only dispatch

Status: accepted (2026-10-27)

## Context

Thirteen tools share a schema/dispatch seam. The risk is drift between the JSON Schema
advertised in `tools/list` and the validation actually applied, and a `dispatch` that grows into
the tools themselves.

## Decision

- **Closed `enum ToolName` + `match`** for dispatch. Adding tool #14 must produce compile errors
  at every place that needs updating. No `struct ToolSpec { build: fn }` table: it forces every
  tool through one uniform closure signature and would only move the match inside.
- **Schema built programmatically in one schema module** from small per-field helpers
  (`int_param`, `enum_param(&[...])`, `str_param`, …). Every enum the schema advertises is the
  same constant the validator checks; no `json!` literal copied per tool.
- **One dispatch seam.** Reads and writes do not split; a write-specific need later is an arm or
  a field, not a second table.
- **`dispatch` is a router.** It never performs HTTP and never formats error text: it routes to
  `tool::<name>::run(&Client, &Args) -> Result<Value, Error>`, and the MCP layer turns `Error`
  into `isError` text. Each tool owns its body.

## Consequences

- Exhaustive matching catches forgotten registrations at compile time.
- A tool that needs more than a builder + a `run` body is a signal the seam is wrong, not a
  reason to widen `dispatch`.
