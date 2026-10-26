# Where the map lives

The map is a single issue on this repo's issue tracker, labelled `wayfinder:map`: the canonical artifact. Its tickets are **child issues** of the map.

**Where the map, its child tickets, blocking, and frontier queries physically live is tracker-specific.** Use the tracker configured in `docs/agents/issue-tracker.md`, and consult its "Wayfinding operations" section for how _this_ repo expresses them.

No tracker is configured in this repo. Default to the **local-markdown tracker**: one file per issue, the map at `.scratch/<feature>/map.md` and its tickets at `.scratch/<feature>/<ticket-name>.md`, or write those files directly with `write`. Do not invent tracker APIs the repo has no tooling for.

If no tracker exists at all, tell the user to run `/skill:setup-matt-pocock-skills`.
