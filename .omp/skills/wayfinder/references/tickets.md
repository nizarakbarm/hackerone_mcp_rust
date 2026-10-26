# Tickets: bodies, types, labels, claiming, blocking

Each ticket is a **child issue** of the map; the tracker's issue id is its identity. Its body is the question, sized to one 100K token agent session:

```markdown
## Question

<the decision or investigation this ticket resolves>
``

The answer isn't part of the body; it's recorded on resolution, as a **resolution comment** on the closed issue. Assets created while resolving a ticket are **linked** from the issue, not pasted in.

## Labels

Each ticket carries a `wayfinder:<type>` label, one of `research`, `prototype`, `grilling`, `task`. `wayfinder:` labels are the only labels a map and its tickets carry — never a triage label like `ready-for-agent`: they are decisions, not implementation work. The body never states the type; read the label.

## Claiming

A session **claims** a ticket by assigning it to the dev driving the map, **first**, before any work, so concurrent sessions skip it. That assignee _is_ the claim: an open, unassigned ticket is unclaimed.

## Resolving

Resolve a ticket as the type its label names, reading the label rather than the body, which never states the type. **Zoom as needed**: fetch the full body of any related or closed ticket on demand, and `read` whichever skills the `## Notes` block names (`skill://<name>`). If in doubt about the type, read `skill://grilling` and read `skill://domain-modeling`.

## Blocking and the frontier

Blocking uses the tracker's **native** dependency relationship — essential, because it renders the frontier _visually_ in the tracker's own UI, so the human sees what's takeable without opening the map. Only a tracker lacking native blocking falls back to a body convention.

A ticket is **unblocked** when every ticket blocking it is closed. The **frontier** is the open, unblocked, unclaimed children: the edge of the known. The map is done when the frontier is empty and no fog is ticketable.

## Ticket types

Every ticket is either **HITL** (human in the loop, worked _with_ a human who speaks for themselves) or **AFK**, driven by the agent alone. A HITL ticket only resolves through that live exchange; the agent never stands in for the human's side of it — a grilling agent that answers its own questions has broken this.

- **Research** (AFK): reading documentation, third-party APIs, or local resources like knowledge bases to surface a fact a decision waits on. Resolved by a subagent (see `skill://wayfinder/references/research-fanout.md`). Use when knowledge outside the current working directory is required.
- **Prototype** (HITL): raise the fidelity of the discussion by making a cheap, rough, concrete artifact to react to — an outline, a rough take, a stub, or UI/logic code. Links the prototype as an asset. Use when "how should it look" or "how should it behave" is the key question.
- **Grilling** (HITL): conversation. The default case: read `skill://grilling` and read `skill://domain-modeling`.
- **Task** (HITL or AFK): manual work that must happen before a _decision_ can be made — nothing to decide, prototype, or research, but the discussion is blocked until it's done. Signing up for a service so its API can be judged, provisioning access, moving data so its shape can be seen. This is the one type that _does_ rather than decides, and it earns its place by unblocking a decision, not by delivering the destination. The agent drives it alone where it can (AFK); otherwise it hands the human a precise checklist (HITL). Resolved when the work is done; the answer records what was done and any resulting facts — credentials location, new URLs, row counts — that later tickets depend on.
