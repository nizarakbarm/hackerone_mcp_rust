---
name: implement-spec
description: "Implement the result of `skill://to-spec` and `skill://to-tickets` in code."
disable-model-invocation: true
---

You have been provided a spec. This spec should have tickets associated with it, describing how to implement the spec.

The issue tracker should have been provided to you. If not, tell the user to run `skill://setup-matt-pocock-skills`.

The goal is the entire spec implemented on a single **integration branch**, with every ticket resolved the way the issue tracker closes work.

The tickets are not a list of steps. They are a **task graph** with blocking relationships between them, so there is always a **frontier** of tickets ready to be grabbed.

Communicate with subagents sparsely, through **context pointers**: the spec, tickets, research notes, and previous commits. Don't duplicate information a pointer already gives.

## Steps

1. Read the spec and tickets to understand the task graph.

2. (optional) Use one **exploration subagent** for any exploration the tickets require — relevant codebase files or external documentation. It must be able to save files: it writes its markdown notes in a directory outside the repo, accessible by all future subagents. That lets **implementer subagents** focus on implementation rather than exploration.

3. Create the integration branch. If the issue tracker closes work through PRs, or the user asks for one, open a draft PR after the first tickets land (a branch with no commits ahead of main can't open one), marked as closing the spec and tickets.

4. Run the ready frontier with **one `task` call**, capped at 4 implementer entries (`agent: task`, `isolated: false` — implementers share the integration branch, so they must not run in separate worktrees). Each entry's brief is self-contained, because children start blank:
   - the ticket to implement, the spec path, and the seams to test at;
   - read `skill://tdd` to build the ticket;
   - commit to the integration branch as the ticket is finished.

5. When a ticket lands, re-check the task graph. Newly unblocked tickets form the next frontier: run it in a new `task` call, again capped at 4.

6. Once all tickets are complete, read `skill://code-review` on the integration branch. Fix all issues raised by the code review in a single **implementer pass**.

7. If a draft PR exists, mark it ready for review. Otherwise, resolve each ticket the way the issue tracker closes work, and report the integration branch.

Write long artifacts (research notes, implementer briefs, reports) to files and reply with the path plus a ≤3-line summary.
