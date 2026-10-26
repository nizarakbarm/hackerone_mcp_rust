# HackerOne API notes

Facts the port depends on. Source of truth: <https://api.hackerone.com/> and the endpoint
behaviour reproduced from upstream `trailofbits/hackerone-mcp`.

## Auth

HTTP Basic: `base64(api_identifier:token)`. The identifier is **not** the login name; it is shown
next to the token value. Tokens are scoped to a program or organization. Base URL:
`https://api.hackerone.com/v1`.

## Endpoints used by the 13 tools

| Tool | Call | `data.type` |
|---|---|---|
| `get_report` | `GET /reports/{id}` | — |
| `get_report_custom_field_values` | `GET /reports/{id}` (normalized client-side) | — |
| `create_report` | `POST /reports` | `report` |
| `query_reports` | `GET /reports` + `filter[...]`, `page[...]`, `sort` | — |
| `list_scopes` | `GET /programs/{id}/structured_scopes` (+ `GET /me/programs` to resolve the id) | — |
| `change_state` | `POST /reports/{id}/state_changes` | `state-change` |
| `create_comment` | `POST /reports/{id}/activities` | `activity-comment` |
| `update_title` | `PATCH /reports/{id}` | `report` |
| `update_reference` | `POST /reports/{id}/issue_tracker_reference_id` | `issue-tracker-reference-id` |
| `update_cves` | `PUT /reports/{id}/cves` | `report-cves` |
| `escalate_report` | `POST /reports/{id}/escalate` | (none — attributes only) |
| `remove_escalation` | `DELETE /reports/{id}/escalate` | — |
| `update_custom_field_values` | `POST /reports/{id}/custom_field_values` (one per attribute) | (none) |

## Values

- Report states: `new`, `pending-program-review`, `triaged`, `needs-more-info`, `resolved`,
  `not-applicable`, `informative`, `duplicate`, `spam`, `retesting`.
- Severity ratings: `none`, `low`, `medium`, `high`, `critical`.
- Sort keys are fully qualified (`reports.created_at`, …); a leading `-` means descending.

## Pagination

`page[number]` (1-based), `page[size]` (1–100). `links.next` is a full URL and is the only
authority for "another page exists" — never `page+1`.

## Rate limits

| Surface | Limit |
|---|---|
| Reads (general) | 600 req/min |
| Report pages | 300 req/min |
| Structured scopes | 50 req/min |
| Writes | 25 req / 20 s |

No retry/backoff in v1 (out of scope per the spec).

## Error body

```json
{ "errors": [ { "status": 403, "title": "…", "detail": "…", "source": { "parameter": "…" } } ] }
```

401 invalid credentials · 403 permissions/IP allowlist/plan · 404 not found · 422 validation.
The client keeps the body verbatim in `Error::Api { status, detail, body }`.
