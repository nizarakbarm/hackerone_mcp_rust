---
name: wayfinder
description: Plan a huge chunk of work (more than one agent session can hold) as a shared map of decision tickets on your issue tracker, resolved one at a time until the way to the destination is clear.
disable-model-invocation: true
---

A loose idea, too big for one session, wrapped in fog: the **destination** isn't visible yet. Wayfinding charts a **shared map** of **decision tickets** — questions resolving to decisions, not slices of a build — worked one at a time until the route is clear. **Name the destination first**: it shapes every ticket and fixes scope.

**Plan, don't do** — decisions, not deliverables. **Refer by name**, never a bare id. **One ticket per session**, research excepted. Long output goes to a file: path plus a ≤3-line summary.

## The map

One issue labelled `wayfinder:map` (the canonical artifact), its tickets child issues. An **index**, not a store: a decision lives only in its ticket.

Read `skill://wayfinder/references/map-body.md`.
Read `skill://wayfinder/references/tickets.md`.
Read `skill://wayfinder/references/fog-of-war.md`.
Read `skill://wayfinder/references/tracker.md`.

## Chart the map

User invokes with a loose idea.

1. **Name the destination.** read `skill://grilling` and read `skill://domain-modeling` to pin the spec, decision, or change.
2. **Map the frontier.** Grill again, **breadth-first** across the whole space, surfacing open decisions and steps takeable now. **No fog?** No map needed: ask the user and stop.
3. **Create the map** (label `wayfinder:map`): Destination and Notes filled, Decisions-so-far empty, fog in **Not yet specified**.
4. **Create the tickets you can specify now** as children, wiring blocking edges and cross-references in a **second pass**: ids come first.
5. **Fire the research subagents**: one `task` call, one entry per `research` ticket, at most 4 — see `skill://wayfinder/references/research-fanout.md`.
6. Stop: charting is one session's work; it resolves nothing.

## Work through the map

User invokes with a map (URL or number). A ticket is **optional**: without one, pick the next decision yourself.

1. Load the **map**: the low-res view, not every ticket body.
2. Choose the ticket — the user's if named, else the first frontier ticket. **Claim it** before any work.
3. Resolve it as the type its `wayfinder:<type>` label names; read the label, the body never states it. **Zoom as needed**: related and closed ticket bodies, and `read` whichever skills the notes name (`skill://<name>`).
4. Record the resolution: a **resolution comment** holding the answer, **close** the issue, **append a context pointer** to Decisions-so-far.
5. Add newly-surfaced tickets; graduate fog the answer made specifiable, clearing each patch from **Not yet specified**. A ticket beyond the destination is **ruled out of scope**, not resolved on the route; invalidated ones get updated or deleted.

Expect concurrent tracker edits.

**Done when the frontier is empty**: no open, unblocked, unclaimed child, no ticketable fog.
