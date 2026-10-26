# ADR-0001: Port trailofbits/hackerone-mcp to Rust with parity-frozen tools

Status: accepted (2026-10-27)

## Context

`trailofbits/hackerone-mcp` is a ~1200-line Python MCP server exposing the HackerOne
report-management API. We rebuild it in Rust. Existing users' muscle memory, prompts, and
agent configs reference its thirteen tool names and argument shapes.

## Decision

- Port to a Rust workspace. Keep the thirteen tools, their names, argument names, enums and
  `inputSchema` structure **as-is**.
- Success payloads stay **raw HackerOne JSON, pretty-printed into `content[0].text`** — no
  `structuredContent`, no normalized envelope.
- Ship exactly these **wart fixes**, nothing else:
  1. `update_custom_field_values` takes `custom_fields: BTreeMap<u32, String>` (typed keys)
     instead of stringly `object` + `_coerce_int_like`.
  2. `update_custom_field_values` returns per-key `{attribute_id, ok, error}` instead of a
     blind list of raw responses.
  3. Delete the `_as_dict` / `_as_list` silent-empty coercion: a malformed API payload is a
     typed decode error, not `{}` / `[]`.
  4. Parameter validation failures are typed `Validation` errors, not ad-hoc `{"error": ...}`
     dicts.
  5. Fix `_resolve_program_id` pagination: follow `links.next`, do not assume `page+1`.
  6. Keep a regression test proving no tool schema exposes a `program` / `team_handle`
     override.
- License: **AGPL-3.0-or-later**. The port is a derivative of AGPL code; README credits
  Trail of Bits.

## Consequences

- Drop-in replacement for existing MCP client configs.
- Upstream quirks that are not on the wart list are preserved on purpose. Changing them later
  is a parity break and needs a new ADR.
- We may not relicense to MIT/Apache without a clean-room rewrite.
