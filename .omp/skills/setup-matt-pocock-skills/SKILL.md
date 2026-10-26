---
name: setup-matt-pocock-skills
description: "Configure this repo for the engineering skills: set up its issue tracker, triage label vocabulary, and domain doc layout. Run once before first use of the other engineering skills."
disable-model-invocation: true
---

# Setup Matt Pocock's Skills

Scaffold the per-repo configuration the engineering skills assume: the **issue tracker** (where issues live), the **triage labels** (strings for the five canonical roles), and the **domain docs** (`GLOSSARY.md` and ADRs, plus the consumer rules).

Prompt-driven, not a deterministic script: explore, present what you found, confirm with the user, then write.

## Process

### 1. Explore

Look at the repo to understand its starting state. Read whatever exists; don't assume:

- `git remote -v` and `.git/config`: is there a remote, and where does it point?
- `AGENTS.md` at the repo root: does it exist? Is there already an `## Agent skills` section in it?
- `GLOSSARY.md`, `GLOSSARY-MAP.md`, `docs/adr/`, and any `src/*/docs/adr/`
- `docs/agents/`: does this skill's prior output already exist?
- `.scratch/`: a local-markdown tracker is already in use
- Is `triage` installed? Try read `skill://triage`; this decides whether Section B runs.
- Monorepo signals: `pnpm-workspace.yaml`, a `workspaces` field in `package.json`, or `packages/*` each with its own `src/`. Absent means single-context, which is almost every repo.

### 2. Present findings and ask

Summarise what's present and what's missing. Then one `ask` per section, in order, leading with the recommended answer so the user can accept it in a word; skip a section exploration already settled (B when `skill://triage` isn't available, C when there's no monorepo). Sections, options, and what to record: `skill://setup-matt-pocock-skills/references/sections.md`.

### 3. Confirm and edit

Draft each for the user to edit before writing:

- the `## Agent skills` block for `AGENTS.md` (template and inclusion rules: `skill://setup-matt-pocock-skills/references/agent-skills-block.md`)
- the contents of `docs/agents/issue-tracker.md`, `docs/agents/domain.md`, and `docs/agents/triage-labels.md` (the last only when `triage` is installed)

### 4. Write

If `AGENTS.md` exists, edit it. If no such file exists, `ask` the user which file to create; don't pick for them. If an `## Agent skills` block is already there, update its contents in place rather than appending a duplicate. Don't overwrite user edits to the surrounding sections.

Seeds in this skill folder: `issue-tracker-local.md` (the default), `-github.md`, or `-gitlab.md` → `docs/agents/issue-tracker.md`; `domain.md` → `docs/agents/domain.md`; and, only when `triage` is installed, `triage-labels.md` → `docs/agents/triage-labels.md` (each as `skill://setup-matt-pocock-skills/<file>`). For "other" trackers, write `docs/agents/issue-tracker.md` from scratch using the user's description.

When Section B ran on GitHub or GitLab, create each configured label the tracker lacks (`gh label create` / `glab label create`).

### 5. Done

Tell the user setup is complete and which skills now read these files. They can edit `docs/agents/*.md` directly later; re-running this skill is only needed to switch trackers or start over.
