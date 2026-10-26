# Outcomes

Step 5 of `skill://triage`. Apply the role the maintainer settled on, then post what that row says.

- `ready-for-agent`: post an agent brief comment (`skill://triage/AGENT-BRIEF.md`).
- `ready-for-human`: same structure as an agent brief, but note why it can't be delegated (judgment calls, external access, design decisions, manual testing).
- `needs-info`: post triage notes (template below).
- For `wontfix`, close the issue, with the comment depending on *why*:
  - **Already implemented**: the change already exists in the codebase. Point to where it lives; do **not** write to `.out-of-scope/` (that KB is for *rejected* requests, not built ones).
  - **Rejected (bug)**: give a polite explanation, then close.
  - **Rejected (enhancement)**: write to `.out-of-scope/`, link to it from a comment, then close (`skill://triage/OUT-OF-SCOPE.md`).
- `needs-triage`: apply the role. Optional comment if there's partial progress.

## Needs-info template

```markdown
## Triage Notes

**What we've established so far:**

- point 1
- point 2

**What we still need from you (@reporter):**

- question 1
- question 2
``

Capture everything resolved during grilling under "established so far" so the work isn't lost. Questions must be specific and actionable, not "please provide more info".
