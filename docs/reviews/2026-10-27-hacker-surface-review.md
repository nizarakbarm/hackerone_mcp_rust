# Code review — v0.2.0 hacker-side surface (2026-10-27)

Diff `git diff v0.1.0...HEAD` (26 files, +2126/−123). Spec = issue #25; ADR-0008 (+amendment).
Axes ran as parallel sub-agents; findings not merged or reranked.

## Standards

Findings, all addressed in `refactor: exhaustive credential class, shared integration-test helpers`:

1. **`err-no-unwrap-prod`** — `ToolName::class()` used `.expect("every ToolName is wired exactly
   once")`, reachable from `dispatch`. Fixed: `class()` is now an exhaustive `match self`, so a new
   variant cannot compile without stating its class, and the registry lost its redundant class
   column (`WIRED` back to `[(ToolName, &str); 20]`). No panic path left in production code; the
   only remaining `expect`s are inside `#[cfg(test)]` blocks (verified: `config.rs:246+`,
   harness `:522+`).
2. **Duplicated Code (test boilerplate)** — `args` / `received` / `query_pairs` / `expected` were
   copied verbatim across `tests/{read_tools,write_tools,hacker_tools,tool_vocabulary,credentials}.rs`.
   Fixed: `tests/common/mod.rs`, `mod common;` per binary.
3. **Shotgun Surgery (tool registry)** — adding a tool touches enum + `name()` + `class()` +
   `WIRED` + `dispatch`. Accepted, not fixed: ADR-0007 chose a closed enum with exhaustive
   matching deliberately, and every extra touch is a compile error, not a silent omission. The
   duplicated class column was removed, which was the only genuinely redundant copy.

## Spec

No missing requirements. Every spec'd tool, argument, limit caveat and credential rule is present,
including: `search_programs` client-side filtering with `pages_scanned`/`truncated`/`next_page`,
the 10 000-item scope cap stated in the description, Lucene `queryString` passed through verbatim,
`MissingCredential` at call time, all 20 tools always advertised.

Out-of-scope list honoured: no `/hackers/reports/{id}`, no `payments/*`, no
`POST /hackers/reports`, no `report_intents`, no credential-dependent tool hiding.

## Worst issue per axis

- Standards: `expect()` on a production path (fixed).
- Spec: none.
