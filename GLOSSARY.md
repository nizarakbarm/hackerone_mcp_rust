# GLOSSARY

Vocabulary for this repo. Use these terms in issues, specs, ADRs, test names, and code identifiers.

## HackerOne domain

- **Report** — a vulnerability submission to a program. Identified by an integer `report_id`. Has a state, severity, title, vulnerability information, impact, and relationships (custom field values, activities, attachments).
- **Program** — the bug bounty program (a.k.a. *team*) that receives reports. Identified by a **program handle** (the slug in `hackerone.com/<handle>`) and an opaque numeric/string **program id** used by some API paths.
- **Program handle** — `HACKERONE_PROGRAM_HANDLE`. The security boundary of this process: every scoped tool call targets exactly this program. Callers can never override it through tool arguments.
- **Structured scope** — an entry in a program's scope table (asset, asset type, instruction, max severity, bounty eligibility). Listed via `/programs/{id}/structured_scopes`.
- **Report state** — lifecycle position of a report. Canonical values: `new`, `pending-program-review`, `triaged`, `needs-more-info`, `resolved`, `not-applicable`, `informative`, `duplicate`, `spam`, `retesting`.
- **Severity rating** — `none` | `low` | `medium` | `high` | `critical`. Distinct from the numeric CVSS score.
- **State change** — the audited transition of a report to a new state, optionally carrying a message, an `original_report_id` (for `duplicate`), or `attachment_ids`.
- **Activity** — an entry on a report timeline. A **comment activity** is user-authored, and is **internal** (team-only) or **external** (visible to the reporter).
- **Custom field attribute** — a program-defined typed field (`attribute_id`, `label`, `field_type`, `internal`, `required`, `regex`, `configuration`).
- **Custom field value** — the value currently set for a custom field attribute on one report. Written one attribute at a time.
- **Escalation** — routing a report to an integration instance (`integration_id`). Reversible.
- **Issue-tracker reference** — the external ticket identifier (e.g. Jira URL) attached to a report.
- **Weakness** — the CWE classification (`weakness_id`) attached to a report.
- **Inbox** — the queue a report lands in; filterable via `inbox_ids`.

## This system

- **MCP tool** — one JSON-Schema-described callable function advertising name, description, `inputSchema`. thirteen exist; their names, arguments, and response payloads are **parity-frozen** against the upstream Python server.
- **Parity** — same tool name, same argument names/enums, same response body shape as `trailofbits/hackerone-mcp`. Response bodies are the raw HackerOne JSON, pretty-printed into `content[0].text`.
- **Wart fix** — a deliberate, listed departure from parity (see ADR-0001). Anything not on that list stays parity.
- **Credential class** — the token pair a tool spends, declared per registry entry: `Customer` (`HACKERONE_API_*`, program side) or `Hacker` (`HACKERONE_HACKER_API_*`, `/v1/hackers/*`). A missing pair fails that tool's call with `MissingCredential`, never startup; a tool can only ever reach the client of its own class (ADR-0008).
- **Transport** — how MCP clients reach the server: `stdio` (default, no auth) or `streamable-http` (bearer-gated).
- **Bearer gate** — the `Authorization: Bearer <token>` check on the HTTP transport, using `HACKERONE_MCP_HTTP_TOKEN`. Always required, including on loopback.
- **api crate** — `crates/hackerone-api`: typed HackerOne REST client + error enum. No MCP types.
- **server crate** — `crates/hackerone-mcp`: tool schemas, dispatch, transports, config. Two binaries: `hackerone-mcp`, `hackerone-mcp-query-reports-harness`.
- **Harness** — live end-to-end checker that spawns the server over stdio, asserts the `query_reports` schema exposes no caller-controlled program field, then runs real filters against a real program.
- **Text passthrough** — success results carry the API JSON as text only; no `structuredContent`.
- **Secret** — a credential value the process holds. Never rendered in formatting: debug and log output show a fixed marker in its place, revealing neither the value nor its length, and no accidental print path exists.
- **Body cap** — the maximum response size one call may read. Enforced while reading, so a chunked or mislabelled response cannot sidestep it.
- **Response too large** — the failure of a call whose success body crossed the body cap. An error body is truncated to its useful head instead of failing, because the status already says more.
