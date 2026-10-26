---
name: writing-for-agents
description: Writing documents for agents. Use when creating or editing skills, or modifying AGENTS.md.
---

Reference for writing any document an agent consumes: a skill, an `AGENTS.md`, a doc reached by a pointer. The packaging differs; the writing does not: the same levers make each one predictable, since the agent takes the same _process_ every run. Write the artifact to a file; reply with its path plus a ≤3-line summary.

## Skills

A skill is `<skills-root>/<name>/SKILL.md` — flat, one directory per skill, no nested grouping. Frontmatter keys: `name`, `description`, optional `globs`, `alwaysApply`, `hide`, `disableModelInvocation` (kebab `disable-model-invocation` accepted), `enabled: false`. A sibling file is read as `skill://<name>/<path>`. Read `skill://writing-for-agents/SKILL-MECHANICS.md` for invocation, router skills, and subagent orchestration.

## Context pointers

A **context pointer** is a reference in the agent's context that names out-of-context material and encodes the condition for reaching it: a skill `description`, a `skill://<name>/<path>` line, a line in `AGENTS.md`. Its _wording_, not its target, decides when and how reliably the agent reaches the material: state what the material is, and list the **branches** that trigger reaching it.

- **Front-load the leading word**: the pointer is where it does its triggering work.
- **One trigger per branch.** Synonyms renaming one branch are one branch written twice.
- **Cut identity the body already carries.**

Weak wording before a must-have target is a variance bug: sharpen it first; inline only if that fails.

## Brevity

Documents spend **context load** (always-loaded material in the window every turn) or **cognitive load** (the human's cost of knowing which documents exist and when to use each). Prune on the **information hierarchy**: inline what every branch needs, disclose what only some reach; one **single source of truth** per meaning; never restate the **environment** (`package.json`, config, layout, `--help`) unless the lookup is expensive; delete whole sentences that fail the **no-op** test. Read `skill://writing-for-agents/references/hierarchy.md`
Read `skill://writing-for-agents/references/pruning.md`

## Voice

A **leading word** is a compact concept already living in the model's pretraining that the agent thinks with (_lesson_, _fog of war_, _tracer bullets_): repeated as a token, never a sentence, it anchors a region of behaviour in the fewest tokens. Reach for an existing word first — a coined one recruits no priors. Hunt restatements to collapse into one token.

Prompt the **positive**: state the target behaviour ("write one-line comments") so the banned one is never spoken; prohibition drags the forbidden behaviour into context and makes it _more_ available. Read `skill://writing-for-agents/references/leading-words.md`

## Steps

Every step ends on a **completion criterion**: can the agent tell done from not-done, and how much does it demand? A vague bound invites **premature completion**. Sharpen the bound first; hide later steps by splitting only across a real context boundary. Read `skill://writing-for-agents/references/steps-and-splitting.md`
