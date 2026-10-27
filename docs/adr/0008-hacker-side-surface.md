# ADR-0008: Hacker-side surface — seven read tools, second credential class

Status: accepted (2026-10-27)

## Context

The shipped 13 tools use a **customer** (program/org) token against
`/v1/reports`, `/v1/programs/{id}/structured_scopes`, `/me/programs`. HackerOne also exposes a
**hacker**-side API (`/v1/hackers/*`), documented in `docs/hacker-api-surface.md`, which answers
questions a triage agent asks constantly: what does this target's scope say, what is excluded,
which weakness IDs exist, what is already disclosed (dupe/prior-art), what did I submit.

It also exposes things that must not be handed to an unattended agent.

## Decision

### In scope — seven read-only tools

| Tool | Endpoint | Note |
|---|---|---|
| `search_programs` | `GET /hackers/programs` | pagination only; bounty/state filtering is **client-side** over a capped scan, and the description says so |
| `get_program` | `GET /hackers/programs/{handle}` | program object incl. `offers_bounties`, `policy`, `submission_state` |
| `get_program_structured_scopes` | `GET /hackers/programs/{handle}/structured_scopes` | `filter[id__gt]`, `created_at__gt`, `updated_at__gt`; documents the 10 000 pagination cap |
| `get_program_scope_exclusions` | `GET /hackers/programs/{handle}/scope_exclusions` | |
| `get_program_weaknesses` | `GET /hackers/programs/{handle}/weaknesses` | |
| `list_my_reports` | `GET /hackers/me/reports` | |
| `search_hacktivity` | `GET /hackers/hacktivity` | `queryString` passed through verbatim (Lucene subset), `sort`, pagination |

### Out of scope, deliberately

- `GET /hackers/reports/{id}` — redundant with the customer-side `get_report` for triage; adds a
  second, differently-shaped report read that would confuse tool selection.
- `GET /hackers/payments/{balance,earnings,payouts}` — human dashboards. An agent never acts on a
  balance read; the credential cost (a second token) already covers a wider surface.
- `POST /hackers/reports` — unattended report submission is the highest-abuse action the binary
  could take. Revisit only on an explicit human request, behind a separate ADR and a
  human-confirmation flow.
- `report_intents` (8 endpoints) — an entire AI report-assistant lifecycle that forces multipart
  upload. Add when a concrete workflow needs it, not speculatively.

### Credentials

- Config gains a second pair: `[hacker_api] username, token` (env
  `HACKERONE_HACKER_API_USERNAME` / `HACKERONE_HACKER_API_TOKEN`), same layering rules as ADR-0004.
- Each tool declares its **credential class** in the registry: `Customer` or `Hacker`. `dispatch`
  selects the client by class; a missing token for the requested class is a typed
  `MissingCredential { class }` error at call time, never at startup.
- Cross-class calls are impossible by construction: the customer client is never reachable from a
  hacker-class tool.

## Consequences

- Tool count goes 13 → 20; parity (ADR-0001) is broken by design, documented in the README
  differences table.
- `tools/list` filtering by available credentials would hide tools and confuse clients, so all 20
  are always advertised; a call without the token fails with a cause-specific error.
- One binary serves both roles. The alternative — a mode switch forcing one credential — was
  rejected because the difference is a per-tool field, and two binaries would duplicate dispatch
  and transports.

## Amendment (2026-10-27): where `CredentialClass` and its error live

Adopted: `CredentialClass` and `Error::MissingCredential { class }` live in **`hackerone-api`**, next
to `Client` and the rest of the error enum.

Rejected: a server-local `ToolError` wrapper with the class defined in `hackerone-mcp`. It avoids an
api-crate concept the transport layer does not act on, but it costs a second error type whose only
job is to re-wrap every existing variant, and it splits the class enum from the client that consumes
it. The api crate already carries policy-shaped variants (`Config`, `Validation`,
`MissingProgramHandle`) for the same reason: one enum, one source of truth, and a `?` that never
needs translation at the boundary. If the api crate ever ships as a standalone client, the class
enum is one more piece of its config surface — not a dependency inversion.
