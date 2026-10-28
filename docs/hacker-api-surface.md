# Hacker-side API surface (evaluated 2026-10-27)

Source: <https://api.hackerone.com/hacker-resources> (HACKERS section, live docs read 2026-10-27).
Everything here requires a **hacker** API token (user settings), not the customer/program token the
13 shipped tools use.

## Inventory

| # | Endpoint | Params | Returns | Status in this repo |
|---|---|---|---|---|
| 1 | `GET /v1/hackers/programs` | `page[number]`, `page[size]` (1–100, default 25) — **no filters, no sort** | `program[]` | shipped as `search_programs` (capped client-side scan) |
| 2 | `GET /v1/hackers/programs/{handle}` | `handle` | `program` | shipped as `get_program` |
| 3 | `GET /v1/hackers/programs/{handle}/structured_scopes` | `handle`, `filter[id__gt]`, `filter[created_at__gt]`, `filter[updated_at__gt]`, pagination (10 000-item cap; page by `id__gt` beyond) | `structured_scope[]` | shipped as `get_program_structured_scopes` (customer-side twin ships as `list_scopes`) |
| 4 | `GET /v1/hackers/programs/{handle}/scope_exclusions` | `handle` | `scope_exclusion[]` | shipped as `get_program_scope_exclusions` |
| 5 | `GET /v1/hackers/programs/{handle}/weaknesses` | `handle`, pagination | `weakness[]` | shipped as `get_program_weaknesses` |
| 6 | `GET /v1/hackers/reports/{id}` | `id` | `report` incl. `reporter`, `assignee`, `program`, `weakness`, `severity`, **`bounties`**, `swag`, `activities`, `attachments`, `structured_scope`, `summaries`. `vulnerability_information` only when the hacker owns it | shipped as `get_hacker_report` (customer-side twin ships as `get_report`) |
| 7 | `GET /v1/hackers/me/reports` | pagination | `report[]` | shipped as `list_my_reports` |
| 8 | `GET /v1/hackers/hacktivity` | `queryString` (Lucene: `severity_rating`, `asset_type`, `substate`, `cwe`, `cve_ids`, `reporter`, `team`, `total_awarded_amount`, `disclosed_at`, `has_collaboration`, `disclosed`), `sort` (default `-latest_disclosable_activity_at`), pagination | `hacktivity_item[]` | shipped as `search_hacktivity` |
| 9 | `GET /v1/hackers/payments/balance` | — | balance object | absent |
| 10 | `GET /v1/hackers/payments/earnings` | pagination | `earning[]` | absent |
| 11 | `GET /v1/hackers/payments/payouts` | pagination | `payout[]` | absent |
| 12 | `POST /v1/hackers/reports` | `data.type`, `attributes.{team_handle,title,vulnerability_information,impact,severity_rating,weakness_id,…}` | `report` | absent |
| 13–18 | `GET·POST /v1/hackers/report_intents`, `GET·PATCH·DELETE /v1/hackers/report_intents/{id}`, `POST …/{id}/submit` | `data.attributes.{team_handle,description}` | `report_intent` | absent |
| 19–21 | `GET·POST /v1/hackers/report_intents/{id}/attachments`, `DELETE …/{id}/attachments/{attachment_id}` | `files[]` **multipart form data** (not JSON) | attachment | absent |

## Facts that shape any implementation

- **No server-side search on the program directory.** Bounty filtering (`offers_bounties`) is
  client-side over pages. Any "find bounty programs" tool must cap its scan and say so in its
  description, or an agent will believe it searched the whole directory.
- **The directory is ~595 programs** (5 pages of 100 + 95, measured 2026-10-27), so a 5×25 default
  budget sees ~21% of it. `search_programs` therefore takes `start_page` and returns `next_page`,
  and resumes from there; it also dedupes by program id because the directory shifts between page
  requests.
- **Program object carries the bounty signals**: `offers_bounties`, `bounty_earned_for_user`,
  `allows_bounty_splitting`, `currency`, `open_scope`, `submission_state`, `state`, `policy`.
- **Structured scopes have the 10 000-item cap**, docs advise `filter[id__gt]` paging beyond it.
- **Report detail is the only bounty-bearing read** besides `payments/*` — `bounties`, `swag` are
  relationships on `/hackers/reports/{id}`.
- **Report intents are the only multipart surface** in the whole API (`files[]` form data).
  Everything else is JSON.
- **Hacker token ≠ customer token.** Same Basic scheme, different credential scope; a hacker token
  cannot triage a program inbox and a customer token cannot read `/hackers/*`.
