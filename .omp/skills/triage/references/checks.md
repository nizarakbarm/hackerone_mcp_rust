# Codebase checks

Run both during step 1 of `skill://triage`, before recommending an outcome.

**(a) Redundancy**: search for an existing implementation of the requested behavior by domain concept (not just the request's wording), and report where you looked. If found, it's an already-implemented `wontfix` (step 5).

**(b) Prior rejection**: read `.out-of-scope/*.md` and surface any that resembles this request (`skill://triage/OUT-OF-SCOPE.md`).

## External PRs in discovery

Discovery surfaces only *external* PRs (the tracker config in `docs/agents/issue-tracker.md` defines who counts as external), so a collaborator's in-flight PR is not triage work. This filter is discovery-only: an explicitly named PR is always triaged regardless of author.
