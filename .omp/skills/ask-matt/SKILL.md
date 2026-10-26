---
name: ask-matt
description: Ask which skill or flow fits your situation. A router over the skills in this repo.
disable-model-invocation: true
---

# Ask Matt

You don't remember every skill, so ask.

Before stating what a skill does or recommending a step be skipped, read that skill's SKILL.md: these summaries are orientation only.

A **flow** is a path through the skills: most run along one **main flow**, two **on-ramps** merge onto it, everything else is standalone or a vocabulary layer underneath. On-ramps, codebase health, vocabulary, standalone skills, setup: `skill://ask-matt/references/routing.md`.

## The main flow: idea → ship

1. **`skill://grill-with-docs`** sharpens the idea by interview. Start here in a **working directory**: it's stateful, retaining what it learns in `GLOSSARY.md` and ADRs. Otherwise use **`skill://grill-me`** — the same `skill://grilling` interview, no paper trail.

2. **Branch: can every question be settled in conversation?** If one needs a runnable answer (state, business logic, a UI you must see), detour through a prototype, bridged by **`skill://handoff`** both ways: hand off out, fresh session against that file, **`skill://prototype`**, hand off the answer back.

3. **Branch: multi-session build?**
   - **Yes** → **`skill://to-spec`** (thread → spec), then **`skill://to-tickets`** (spec → tracer-bullet tickets declaring their **blocking edges**). Then either **`skill://implement`** per ticket, **`/clear`ing between tickets**, or **`skill://implement-spec`** for the whole spec in one run (tickets as a **task graph**, implementer subagents across the ready **frontier**, one **integration branch**).
   - **No** → **`skill://implement`** here, in the same context window.

   Either way the code is built by **`skill://tdd`** (one red-green slice at a time) and closed out by **`skill://code-review`** (axes: Standards, Spec). **`skill://pr`** shapes the PR body.

4. **`skill://retro`** closes the loop: looks back over the session and suggests changes to the agent's **environment**, not the code — pointers, checks, standards, steering files, tooling.

### Context hygiene

Keep steps 1–3 in **one unbroken context window** (don't compact or clear until after `skill://to-tickets`) so grilling, spec, and tickets share the same thinking; each `skill://implement` starts fresh from its ticket. Run `skill://retro` in the session it looks back on, before you clear; after clearing, point it at that log.

The limit is the **[smart zone](https://www.aihero.dev/ai-coding-dictionary/smart-zone)**, ~150k tokens. If a session approaches it before `skill://to-tickets`, `/compact` at the nearest phase boundary rather than pushing on degraded.

## Phase boundaries

A **phase** is a chunk of work inside a session: the grilling, the implementation, the QA. At the **boundary** between two you have five options: **Continue**, **`/clear`**, **`skill://handoff`**, **Subagent** (`task`), and **`/compact`** (the default, at the bottom of the tree, not the first reach). Decide **at** a boundary; mid-phase, continue or split the rest into subagents. `skill://ask-matt/PHASE-BOUNDARIES.md` has the ordered tree, each branch's reasoning, and why **Continue** is the one to rule out first.
