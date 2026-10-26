# Research fan-out

Fired by **Chart the map**, step 5. Resolves every `research` ticket created in step 4, in parallel.

One `task` call with **one entry per research ticket, capped at 4**; more tickets than that queue. Each entry:

- `agent: sonic` — pure reading and reporting, no design decisions.
- A **self-contained brief**: children start blank, so paste the ticket body plus the question it waits on, the repo paths or URLs to read, and what a usable answer looks like. Include the map's Destination line so the child knows why the fact matters.
- `solutionSpace`: fact-finding only — the answer is a documented fact, not a decision.
- `isolated: true` only if the reading needs a worktree; plain reads don't.

Findings go to a **cited Markdown file** (every claim carrying its source path or URL), written to the local-markdown tracker directory — `.scratch/<feature>/research/<ticket-name>.md` — or the tracker's own attachment convention. The ticket gets a **context pointer** to that file; the file is never pasted into the ticket, and no PR is opened or merged. Push a throwaway `research/<name>` branch only if the findings must be shared.

Long findings live in the file. Report each file's path to the map, not its contents.
