# The map body

The whole map at low resolution, loaded once per session. Open tickets are **not** listed: they are open child issues, found by query. The map is an **index**, not a store: a decision lives in exactly one place, its ticket, so the map never restates it, only gists it and links.

```markdown
## Destination

<what reaching the end of this map looks like: the spec, decision, or change this effort is finding its way to. One or two lines; every session orients to it before choosing a ticket.>

## Notes

<domain; skills every session should consult; standing preferences for this effort>

## Decisions so far

<!-- the index: one line per closed ticket, enough to judge relevance, then zoom the link for the detail the ticket holds -->

- [<closed ticket title>](link): <one-line gist of the answer>

## Not yet specified

<!-- see "Fog of war": in-scope fog you can't ticket yet; graduates as the frontier advances -->

## Out of scope

<!-- see "Out of scope": work ruled beyond the destination; closed, never graduates -->
``

Every map and ticket is an issue, so it has a **name**: its title. In everything the human reads (narration, Decisions so far), refer to it by that name, never by a bare id, number, or slug. A wall of `#42, #43, #44` is illegible; names read at a glance. The id and URL don't vanish — a name wraps its link, but they ride _inside_ the name, never stand in for it.
