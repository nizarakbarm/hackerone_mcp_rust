---
name: retro
description: "Conduct a retrospective on a coding session."
disable-model-invocation: true
---

The user has asked for a **retrospective**: improvements to the coding agent's **environment** for future runs.

## Steps

1. Read `skill://writing-for-agents` for the writing style guide.

2. Read the primary sources for the session the user specifies — possibly session logs on this machine; default to the current one.

3. Look for candidates for improvement in these categories.

- **Navigation**: how easy was it to find the right files? Hidden dependencies between files? Would a **navigation pointer** make it easier? _Use when_ the session took a long time to find a piece of information.
- **Automated checks**: could a check catch errors the agent made (linting, typing, tests, filesystem linters)? Read the repo's own check command first (its `package.json`/build-tool `lint`/`check` scripts, its CI workflow), so a check that sits unwired or silently broken is the finding, not a reinvention. A repo with no **guardrail** (no pre-commit hook, no CI job running its lint/typecheck/test command) is itself a finding. _Use when_ the agent made a mistake a check could have caught, or the repo has no guardrail at all.
- **Coding standards**: should the **reviewer agent** get a new rule to enforce? Should an existing rule be removed or clarified? Classify the violation first: a **mechanical** one (fixed syntactic pattern, banned API, import shape, file-location rule) gets a deterministic check, full stop — a custom rule in the repo's own linter, a new pre-commit hook, or a new CI job, whichever is cheapest for the repo's language and guardrail. Default to the check over the rule. Reserve `CODING_STANDARDS.md` for genuine **judgement calls** (cross-file consistency, "matches the surrounding style", anything no guardrail could substitute for). _Use when_ the reviewer agent failed to catch a mistake.
- **Global AGENTS.md**: steering instructions that should move to coding standards (or automated checks) instead? _Use when_ the AGENTS.md file is particularly large — in the repo OR the user's global scope.
- **Tool economy**: expensive or token-inefficient tool calls (CLI's, MCP's) that could be streamlined? _Use when_ the agent made an expensive tool call.
- **No-ops**: instructions in steering files that don't modify the agent's behavior. _Use when_ the steering files are large and unwieldy.
- **Information access**: opportunities to increase the agent's access to information, such as teeing dev server logs or readonly access to third-party services. _Use when_ a crucial piece of information was not available.

4. Present these candidates to the user, in order of severity.

Read `skill://retro/references/environment-files.md` for what each steering file is for, and why standards belong to the review stage.

Write the candidate list to a file and reply with the path plus a ≤3-line summary.
