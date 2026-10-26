# ADR-0004: Layered configuration, field-wise precedence

Status: accepted (2026-10-27)

## Context

Upstream reads three environment variables and nothing else, which makes multi-client setup
and `--config`-driven testing awkward. Every credential is a per-process value; the program
handle is the security boundary.

## Decision

Sources, highest precedence first:

1. `--config <path>` (explicit file; error if unreadable)
2. CLI flags (`--program-handle`, `--api-username`, `--api-token`, `--base-url`, `--bind`,
   `--transport`)
3. Environment: `HACKERONE_API_USERNAME`, `HACKERONE_API_TOKEN`,
   `HACKERONE_PROGRAM_HANDLE`, `HACKERONE_API_BASE_URL`, `HACKERONE_MCP_HTTP_TOKEN`
4. `./hackerone-mcp.toml` (cwd only — no ancestor walk)
5. `$XDG_CONFIG_HOME/hackerone-mcp/config.toml` (fallback `~/.config/...`)

- Merge is **per field**: a later source only fills fields the earlier sources left unset.
- TOML shape: `[api] username | token | program_handle | base_url`, `[http] bind | token`.
- Secret files must not be world-readable: when the loaded config file contains a token,
  refuse to run if its mode has group/other bits set (Unix).
- Unknown keys in the config file are an error (`deny_unknown_fields`), so typos don't
  silently fall back to env.
- `program_handle` is single-valued per process. No profiles, no multi-program routing.

## Consequences

- Test fixtures can pin config with `--config` and ignore the developer's environment.
- Adding a field means touching five places; keep the struct+merge in one module.
