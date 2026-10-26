# Code review — 2026-10-27

Two-axis review of `git diff 76f0c1f...HEAD` (commit `dbae5cf`, 40 files, +10155).
Axes ran as parallel sub-agents; findings are not merged or reranked.

## Standards

No substantive standard violations. Verified against ADR-0001…0007:

- ADR-0001 — all six wart fixes present (typed `BTreeMap<u32, String>` custom fields, per-key
  `{attribute_id, ok, error}`, no `{}`/`[]` coercion, typed `Validation`, `links.next`
  pagination, scope-regression test).
- ADR-0002 — crate split honoured: `hackerone-api` has no MCP types.
- ADR-0003 — `thiserror` enum only; no `anyhow` in source; no silent coercion.
- ADR-0004 — six-source layered config, `deny_unknown_fields`, 0600 secret check.
- ADR-0005 — dual transport, bearer gate with no loopback exemption, non-loopback warning.
- ADR-0006 — edition 2024, rustls + OS trust store, stderr-only tracing.
- ADR-0007 — closed `ToolName` enum, exhaustive dispatch, shared schema helpers.

Baseline smells noted as intentional: `Params { query, body }` data clump (the parity pattern),
closed `switch` on `ToolName` (ADR-0007 by design). `.expect()`/`.unwrap()` only in `#[cfg(test)]`.

## Spec

- Missing/partial: one flag — user story #4 says "ten time windows". Verified in
  `crates/hackerone-mcp/src/tool/mod.rs`: `QUERY_REPORT_DATE_FILTERS` has exactly 10 entries
  (created/submitted/triaged/closed/last_activity × after/before). **Not a defect.**
- Scope creep: none; nothing from the Out-of-Scope list appears in the diff.
- Implemented-but-wrong: none found.

## Worst issue per axis

- Standards: none.
- Spec: none after verifying the date-window count.
