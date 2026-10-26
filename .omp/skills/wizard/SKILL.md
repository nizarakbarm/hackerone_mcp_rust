---
name: wizard
description: Generate an interactive bash wizard walking a human through steps only they can perform. Use when provisioning infrastructure, setting up credentials or CI secrets, walking an unfamiliar third-party dashboard, or running a one-off migration or cutover. Skip steps the agent can perform itself.
---

# Wizard

A **wizard** is a bash script that walks a human through a manual procedure: tedious by hand, tedious to re-explain to an AI every time. It opens each URL, says what to click and copy, captures the values, writes them where they belong (`.env`, GitHub secrets), confirms each stage, counts the stages left. Only **human-only steps** belong in it: anything `bash`, `write` or `edit` can do, you do.

The UX is already solved by `skill://wizard/template.sh`: stage progress, gates, cross-platform URL opening (incl. WSL), hidden secret entry, idempotent `.env` upserts, `gh secret`/`gh variable` writes, closing summary. **Your job is only to scope the procedure and author its stages.** The library above the `STAGES` marker is identical in every wizard; that consistency is the point: never hand-edit it.

Ephemeral by default: built for one run, saved to a scratch or `scripts/` path, deleted after. Commit it only for a repeatable setup path that belongs in the repo.

## Process

### 1. Scope the procedure

Work out every manual step the human must take and every value captured. Read the repo first, don't ask cold: what to read, and what scoping must end with, is in `skill://wizard/references/scoping.md`. Then show the ordered stages and their values and confirm: they may add, drop, or reorder.

### 2. Map each stage's journey

For each stage, write the precise path a human follows: the URL to open, what to do there, where the value shows, which variable it fills — e.g. "Dashboard → Developers → API keys → Reveal test key → copy". Don't know the UI or the exact command? Ask, or check the docs: never invent steps that may not exist. Every stage must trace to instructions a stranger could follow.

### 3. Author the wizard

Copy `template.sh` to the target path with `write`; replace the example with one `stage` per step, in dependency order, and set `TOTAL_STAGES`. Helpers and authoring conventions: `skill://wizard/references/authoring.md`.

### 4. Verify and hand off

- `bash -n <script>`; `shellcheck` if available; `chmod +x <script>`.
- Don't run it end-to-end yourself: it opens browsers and blocks on human input. Trace it statically: every value from step 1 lands where step 1 said, and every `set_secret` name matches a `secrets.*` reference in CI.
- Tell the user how to run it. Repeatable setup path? Commit it and link it from the README, so the next person runs the script, not an AI.
