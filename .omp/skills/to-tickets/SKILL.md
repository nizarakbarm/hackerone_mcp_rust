---
name: to-tickets
description: Break a plan, spec, or the current conversation into tracer-bullet tickets, each declaring its blocking edges, published to the configured tracker (local files: edges as text, one file per ticket; real tracker: native blocking links).
disable-model-invocation: true
---

# To Tickets

Break a plan, spec, or conversation into **tickets**: tracer-bullet vertical slices, each declaring what **blocks** it.

Tracker convention: `docs/agents/issue-tracker.md`. Label vocabulary: `docs/agents/triage-labels.md`. If either is missing, tell the user to run `/skill:setup-matt-pocock-skills`.

## Process

### 1. Gather context

Work from what's already in context. If the user passes a reference (a spec path, an issue number or URL), fetch it and read the full body and comments.

### 2. Explore the codebase (optional)

Explore the codebase if you haven't already. Ticket titles and descriptions use the project's domain glossary vocabulary and respect ADRs in the area you're touching. Look for opportunities to prefactor: "Make the change easy, then make the easy change."

### 3. Draft vertical slices

Each slice cuts a narrow but COMPLETE path through every layer (schema, API, UI, tests): vertical, NOT a horizontal slice of one layer. A completed slice is demoable or verifiable on its own, and sized to fit one fresh context window. Prefactoring comes first.

Give each ticket its **blocking edges**: the other tickets that must complete before it can start. A ticket with no blockers can start immediately.

**Wide refactors are the exception to vertical slicing.** If a slice is really one mechanical change whose **blast radius** fans across the whole codebase, don't force it into a tracer bullet: sequence it as **expand–contract**. Read `skill://to-tickets/references/wide-refactors.md` before drafting its tickets.

### 4. Quiz the user

Present the breakdown as a numbered list; for each ticket show its **Title**, what blocks it, and **What it delivers** (the end-to-end behaviour it makes work). Then ask:

- Does the granularity feel right? (too coarse / too fine)
- Are the blocking edges correct: does each ticket only depend on tickets that genuinely gate it?
- Should any tickets be merged or split further?

Iterate until the user approves the breakdown.

### 5. Publish the tickets

Publish the approved tickets to the tracker `docs/agents/issue-tracker.md` documents; the tickets are the same either way, only the shape of the blocking edges changes. Mechanics and both templates: `skill://to-tickets/references/publishing.md`. A local markdown tracker is `write` calls, never `gh`.

Apply the `ready-for-agent` triage label unless instructed otherwise; the tickets are agent-grabbable by construction. Work the **frontier**: any ticket whose blockers are all done. For a purely linear chain that means top to bottom. Do NOT close or modify any parent issue.

Reply with the paths or numbers plus a ≤3-line summary.
