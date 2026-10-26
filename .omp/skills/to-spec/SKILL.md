---
name: to-spec
description: "Turn the current conversation into a spec and publish it to the project issue tracker: no interview, just synthesis of what you've already discussed."
disable-model-invocation: true
---

Synthesize a spec from the current conversation and your understanding of the codebase. Do NOT interview the user; just synthesize what you already know.

The issue tracker and triage label vocabulary should have been provided to you: `docs/agents/issue-tracker.md` and `docs/agents/triage-labels.md`. If not, tell the user to run `/skill:setup-matt-pocock-skills`.

## Process

1. Explore the repo to understand the current state of the codebase, if you haven't already. Use the project's domain glossary vocabulary throughout the spec, and respect any ADRs in the area you're touching.

2. Sketch out the seams you're going to test the feature at. Existing seams should be preferred to new ones. Use the highest seam possible. If new seams are needed, propose them at the highest point you can. The fewer seams across the codebase, the better — the ideal number is one. Check with the user that the seams match their expectations.

3. Write the spec using `skill://to-spec/references/spec-template.md`, then publish it per the tracker convention in `docs/agents/issue-tracker.md`: on a local markdown tracker `write` it to `.scratch/<feature-slug>/spec.md` (no `gh` calls); on GitHub or GitLab, `gh issue create` / `glab issue create`. Apply the `ready-for-agent` triage label — no need for additional triage.

Keep the spec in its file or issue. Reply with its path or number plus a ≤3-line summary.
