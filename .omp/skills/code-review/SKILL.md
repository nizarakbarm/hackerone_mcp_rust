---
name: code-review
description: "Review the changes since a fixed point (commit, branch, tag, or merge-base) on two axes: Standards (does the code follow this repo's documented coding standards?) and Spec (does it match what the originating issue/spec asked for?). Runs both reviews as parallel sub-agents and reports them side by side. Use when the user wants to review a branch, a PR, work-in-progress changes, or asks to \"review since X\"."
---

Two-axis review of the diff between `HEAD` and a user-supplied fixed point:

- **Standards**: does the code conform to this repo's documented standards?
- **Spec**: does it faithfully implement the originating issue / spec?

Both axes run as **parallel sub-agents** so they don't pollute each other's context; this skill aggregates their findings. Fixes stay with the parent — the reviewers only report.

Issue tracker should have been provided; if not, tell the user to run `skill://setup-matt-pocock-skills`.

## Setup

1. **Pin the fixed point**: whatever the user said (commit SHA, branch, tag, `main`, `HEAD~5`), or ask. Capture `git diff <fixed-point>...HEAD` (three-dot, so the comparison is against the merge-base) and `git log <fixed-point>..HEAD --oneline`. Confirm `git rev-parse <fixed-point>` resolves and the diff is non-empty here: a bad ref or empty diff must fail here, not inside two parallel sub-agents.

2. **Identify the spec source**: read `skill://code-review/references/spec-source.md` for the ordered fallbacks. No spec at all → the **Spec** axis skips and reports "no spec available".

3. **Identify the standards sources**: search the repo for every file documenting how code should be written; if `CODING_STANDARDS.md` or `CONTRIBUTING.md` exists, it must be on the list. The Standards axis also always carries the **smell baseline** (Fowler, _Refactoring_ ch.3), which applies even with no repo docs — read `skill://code-review/references/smells.md`.

## 4. Run both axes in one `task` call

Issue **one `task` call with two entries**, both `agent: reviewer`, both in the foreground, and aggregate their reports. Give each entry its own `solutionSpace` (Standards: conformance to repo standards; Spec: fidelity to the spec). Children start blank, so each brief is self-contained: diff command, commit list, that entry's material.

Read `skill://code-review/references/subagent-briefs.md` for the two brief texts — Standards carries the smell baseline pasted in full, Spec carries the spec.

If the `task` tool is unavailable, run both axes yourself, sequentially, keeping findings separate.

## 5. Aggregate

Present the two reports under `## Standards` and `## Spec` headings, verbatim or lightly cleaned. Do **not** merge or rerank findings: the axes are deliberately separate (read `skill://code-review/references/two-axes.md`).

End with one line: total findings per axis and the worst issue _within each axis_ (if any) — never a single winner across axes.

Write the aggregated report to a file and reply with the path plus a ≤3-line summary.
