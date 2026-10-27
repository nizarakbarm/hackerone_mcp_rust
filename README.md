# HackerOne MCP Server (Rust)

A [Model Context Protocol](https://modelcontextprotocol.io) server for the
[HackerOne](https://docs.hackerone.com/en/articles/8404400-hackerone-api)
report-management API. Drop it into Codex, Claude Code, or any other
MCP-compatible client to triage, comment on, and update HackerOne reports from
inside your agent.

This is a Rust port of
[`trailofbits/hackerone-mcp`](https://github.com/trailofbits/hackerone-mcp).
The thirteen tool names and argument schemas are kept for drop-in
compatibility; the deliberate differences are listed
[below](#differences-from-upstream).

## Tools

All paths are relative to `https://api.hackerone.com/v1`.

| Tool | HackerOne endpoint | What it does |
|---|---|---|
| `get_report` | `GET /reports/{report_id}` | Fetch one report by ID. |
| `get_report_custom_field_values` | `GET /reports/{report_id}` | Read the custom field values currently set on one report, normalized with attribute IDs and labels. |
| `create_report` | `POST /reports` | Import a known vulnerability into the configured program. |
| `query_reports` | `GET /reports` | Query paginated reports for the configured program with filters, sort, and paging. |
| `list_scopes` | `GET /me/programs`, then `GET /programs/{program_id}/structured_scopes` | List structured scopes for the configured program with filters and paging. |
| `change_state` | `POST /reports/{report_id}/state_changes` | Move a report between states (triaged, resolved, duplicate, …). |
| `create_comment` | `POST /reports/{report_id}/activities` | Add an internal or external comment. |
| `update_title` | `PATCH /reports/{report_id}` | Rename a report. |
| `update_reference` | `POST /reports/{report_id}/issue_tracker_reference_id` | Set the issue-tracker reference (for example a Jira URL). |
| `update_cves` | `PUT /reports/{report_id}/cves` | Attach CVE IDs. |
| `escalate_report` | `POST /reports/{report_id}/escalate` | Escalate a report with an integration ID. |
| `remove_escalation` | `DELETE /reports/{report_id}/escalate` | Remove an escalation. |
| `update_custom_field_values` | `POST /reports/{report_id}/custom_field_values` | Set custom field values from a typed `custom_fields` map; returns a per-key result. |
| `search_programs` | `GET /hackers/programs` | Scan up to 20 directory pages (default 5) and filter `offers_bounties`/`state`/`submission_state` client-side; reports truncation when the budget ended with pages left. |
| `get_program` | `GET /hackers/programs/{handle}` | Fetch one program by handle (bounty signals, policy, submission state). |
| `get_program_structured_scopes` | `GET /hackers/programs/{handle}/structured_scopes` | List a program's structured scopes; documents the 10 000-item API cap and `scope_id_gt` paging beyond it. |

`query_reports` and `list_scopes` always scope results to the configured
program handle; no tool accepts a program/team-handle argument. `list_scopes`
resolves the handle to a program ID by following `links.next` on
`/me/programs`. The three `hacker` tools take an explicit `handle` and spend
the hacker token pair, not the customer pair.

## Prerequisites

- Rust 1.88 or newer (for `cargo install`).
- A HackerOne API username (the identifier shown next to the token, **not**
  your login name) and token, authorized for the target program.
- A program handle — the last path segment of `hackerone.com/<handle>`. For
  testing, create a free sandbox program at
  [hackerone.com/teams/new/sandbox](https://hackerone.com/teams/new/sandbox).

## Install

```bash
cargo install --git https://github.com/nizarakbarm/hackerone_mcp_rust.git hackerone-mcp
```

This installs both `hackerone-mcp` and the `hackerone-mcp-query-reports-harness`
binary. A local checkout works too:

```bash
cargo install --path crates/hackerone-mcp
```

### First run

Run the setup wizard once. It walks you through creating a sandbox program,
captures the API credentials, writes
`$XDG_CONFIG_HOME/hackerone-mcp/config.toml` (default
`~/.config/hackerone-mcp/config.toml`) with mode `0600`, and verifies the
credentials with `GET /me/programs`:

```bash
./scripts/setup.sh
```

On Unix, the server refuses to load a token-bearing config file that is
group/world readable.

### MCP client configuration (stdio, the default)

```json
{
  "mcpServers": {
    "hackerone": {
      "command": "hackerone-mcp",
      "args": ["--transport", "stdio"],
      "env": {
        "HACKERONE_API_USERNAME": "your-api-username",
        "HACKERONE_API_TOKEN": "your-api-token",
        "HACKERONE_PROGRAM_HANDLE": "your-program-handle"
      }
    }
  }
}
```

Check the binary directly:

```bash
hackerone-mcp --version
hackerone-mcp --help
```

## Transports

### stdio (default)

`--transport stdio`. MCP speaks JSON-RPC on stdin/stdout; all logs go to stderr,
so stdout stays protocol-clean. There is no authentication layer: the process
is reachable only by whoever started it.

### Streamable HTTP

```bash
export HACKERONE_MCP_HTTP_TOKEN="$(openssl rand -hex 32)"
hackerone-mcp --transport http
```

- Endpoint: `http://127.0.0.1:8765/mcp`.
- Default bind address: `127.0.0.1:8765`; override with `--bind ADDR` or
  `[http] bind`.
- **Every** request must send `Authorization: Bearer <HACKERONE_MCP_HTTP_TOKEN>`.
  Missing or wrong tokens get a bare `401` with `WWW-Authenticate: Bearer`,
  before any MCP session is created. There is no loopback exemption. If the
  variable is unset, `--transport http` refuses to start.
- The bearer gate is a deploy gate, not transport security: there is no
  in-process TLS. On loopback that is fine; for anything else, bind to a
  loopback address behind a TLS-terminating reverse proxy. `--bind` to a
  non-loopback address logs a warning at startup.
- The upstream HackerOne credentials come from the configuration layers below,
  not from the incoming `Authorization` header. One process serves one customer
  identity, one optional hacker identity, and one program.

## Configuration

Precedence, highest first, merged per field: `--config <path>` > CLI flags >
environment variables > `./hackerone-mcp.toml` > `$XDG_CONFIG_HOME/hackerone-mcp/config.toml`
(fallback `~/.config/hackerone-mcp/config.toml`).

CLI flags: `--config`, `--api-username`, `--api-token`, `--program-handle`,
`--base-url`, `--bind`, `--transport`.

| Variable | Meaning |
|---|---|
| `HACKERONE_API_USERNAME` | Customer (program-side) API identifier, HTTP Basic username. Required by the customer-side tools; when missing, those calls fail with `missing customer credential`. |
| `HACKERONE_API_TOKEN` | Customer API token, HTTP Basic password. |
| `HACKERONE_HACKER_API_USERNAME` | Hacker-side API identifier (`/v1/hackers/*`). Required only by hacker-class tools. |
| `HACKERONE_HACKER_API_TOKEN` | Hacker-side API token. |
| `HACKERONE_PROGRAM_HANDLE` | Program handle; required by `create_report`, `query_reports`, and `list_scopes`. |
| `HACKERONE_API_BASE_URL` | API root for both credential classes, default `https://api.hackerone.com/v1`. |
| `HACKERONE_MCP_HTTP_TOKEN` | Bearer token for `--transport http`; required there. |
| `RUST_LOG` | `tracing` filter, default `info`. Logs always go to stderr. |

All thirteen shipped tools spend the **customer** pair. A tool whose class has no
configured pair is still advertised by `tools/list`; calling it returns
`isError` with the missing environment variables named. Startup never requires
credentials.

Config file shape (unknown keys are rejected):

```toml
[api]
username = "your-api-username"
token = "your-api-token"
program_handle = "your-program-handle"
# base_url = "https://api.hackerone.com/v1"

[hacker_api]
# username = "your-hacker-username"
# token = "your-hacker-token"

[http]
# bind = "127.0.0.1:8765"
# token = "http-bearer-token"
```

## Live `query_reports` harness

`hackerone-mcp-query-reports-harness` starts the server over stdio, checks that
`query_reports` is advertised without a caller-controlled program field, then
calls it against the real API:

```bash
HACKERONE_API_USERNAME=... \
HACKERONE_API_TOKEN=... \
HACKERONE_PROGRAM_HANDLE=... \
hackerone-mcp-query-reports-harness \
  --state triaged \
  --page-size 10 \
  --min-results 1
```

Add `--raw` for the full JSON payload. Other flags: `--page-number`,
`--sort-by`, `--asc`/`--desc`, `--severity`, `--report-id`, `--reporter`,
`--assignee`, `--inbox-id`, the date windows (`--created-after`,
`--submitted-before`, `--triaged-after`, `--closed-before`,
`--last-activity-after`, …), `--summary-limit`, and `--min-results`.

## Differences from upstream

| Area | Upstream (`trailofbits/hackerone-mcp`) | This port |
|---|---|---|
| Tool errors | Failures are returned as ordinary text content shaped `{"error": {"status": …, "detail": …}}`; `isError` is never set. | Failures set MCP `isError: true`; the text is the typed error message. |
| Custom-field writes | Stringly `object` values go through `_coerce_int_like`; the result is a blind list of raw responses. | `custom_fields: {attribute_id: value}` is a typed map, and the result is a per-key `{attribute_id, ok, error}` list. |
| Malformed API payloads | `_as_dict` / `_as_list` silently substitute `{}` / `[]`. | A malformed payload is a typed decode error; nothing is silently emptied. |
| `/me/programs` pagination | Increments `page[number]`, which can skip or repeat programs. | Follows the API's `links.next` cursor. |
| Configuration | Three environment variables only. | Five layers (`--config`, flags, env, project TOML, XDG config), merged per field, with a Unix permission check on secret files. |
| Transport | stdio only. | stdio plus bearer-gated streamable HTTP on `127.0.0.1:8765/mcp` by default. |
| Tracing / redaction | Python logging. | `tracing` to stderr with an `RUST_LOG` filter; no credential, token, or `Authorization` value appears in a log statement. |
| AGPL attribution | AGPL-3.0-or-later, Trail of Bits. | Same license; this port credits Trail of Bits in this README and ships the upstream license text. |
| Hacker-side surface | none. | **Partly shipped:** the three program reads (`search_programs`, `get_program`, `get_program_structured_scopes`) behind a second credential class; the remaining four read-only `/hackers/*` tools are still planned — see [ADR-0008](docs/adr/0008-hacker-side-surface.md) and [docs/hacker-api-surface.md](docs/hacker-api-surface.md), which also record what was rejected and why. |

## Development

```bash
./scripts/check.sh   # fmt + clippy -D warnings + workspace tests; CI runs exactly this
```

API facts the tools depend on (auth, endpoints, `links.next`, rate limits, error body):
[`docs/hackerone-api.md`](docs/hackerone-api.md). Decisions: [`docs/adr/`](docs/adr/).

## Security

- Credentials are read from configuration only; they are never logged or
  written by the server. The setup script writes the config as `0600`.
- The default API root is HTTPS, verified with `rustls` and the OS trust store;
  no OpenSSL is linked.
- Treat the HackerOne token as a secret with the same reach as your HackerOne
  account, and treat `HACKERONE_MCP_HTTP_TOKEN` as the key to every
  write-capable tool on that port.

## License

AGPL-3.0-or-later. The license text in [`LICENSE`](LICENSE) is verbatim from
the upstream project. This Rust port is a derivative work of
[`trailofbits/hackerone-mcp`](https://github.com/trailofbits/hackerone-mcp),
which is also AGPL-3.0-or-later; credit for the original design belongs to
Trail of Bits.
