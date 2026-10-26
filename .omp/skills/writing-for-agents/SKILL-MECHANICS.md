# Skill mechanics

The skill-specific branch of `skill://writing-for-agents`: what changes when the document is a skill (layout, frontmatter, the invocation choice, router skills, subagent orchestration). Everything else about writing it is the universal reference in `SKILL.md`.

## Layout and frontmatter

- A skill is `<skills-root>/<name>/SKILL.md`: one directory per skill, flat. Nested grouping (`<skills-root>/group/<name>/SKILL.md`) never loads.
- Frontmatter keys: `name`, `description`, and the optional `globs`, `alwaysApply`, `hide`, `disableModelInvocation` (kebab `disable-model-invocation` is accepted), `enabled: false`.
- Any file beside `SKILL.md` is disclosed reference, read as `skill://<name>/<path>` — e.g. `skill://writing-for-agents/references/pruning.md`. Point at it with the same one-line pointer form as any other disclosed reference.

## Invocation

Two choices, trading the two loads:

- A **model-invoked** skill keeps a `description`, so the agent can fire it autonomously, and other skills can reach it. You can still type its name: model-invocation always _includes_ user reach; a description only ever adds agent discovery, never removes the human's. The description is the skill's top-level context pointer, forced to stay loaded at all times: permanent context load in exchange for discoverability. A model-invoked skill whose content is all reference is also one home for shared reference: another skill can invoke it, so reference needed by several skills lives in one place. Mechanics: omit `disableModelInvocation`, and write a model-facing description carrying the trigger branches (the pointer-writing rules in `SKILL.md` apply in full).
- A **user-invoked** skill strips the description from the agent's reach: the model never auto-fires it, only the human typing `/skill:<name>` invokes it, and it spends cognitive load — you are the index that must remember it exists. Mechanics: set `disable-model-invocation: true`; the `description` becomes human-facing: a one-line summary, trigger lists stripped. It is still a file another skill may point at with `read skill://<name>`; that is a deliberate reference, not agent discovery.

Pick model-invocation only when the agent must reach the skill on its own, or another skill must. If it only ever fires by hand, make it user-invoked and pay no context load.

## Splitting by invocation

The invocation cut of splitting (the sequence cut lives in `SKILL.md`): split off a model-invoked skill when you have a distinct leading word that should trigger it on its own (a trigger word you actually use in your prompts), or another skill must reach it. You pay context load for the new always-loaded description, so that independent reach has to be worth it.

## Router skills

When user-invoked skills multiply past what you can remember, that piled-up cognitive load is cured by a **router skill**: one user-invoked skill that names the others and when to reach for each, so the human has one skill to remember instead of many. Have it name each skill as `/skill:<name>` for the human and `read skill://<name>` for the agent following the router.

## Subagent orchestration

When a skill dispatches parallel work, name the concrete call: `task` with N entries (`tasks[]`, `agent: scout|task|reviewer|sonic`), fan-out capped at 4. Children start blank, so the brief carries paths, requirements, and acceptance criteria. Read-only investigation belongs in `agent: scout`. `isolated: true` gives an entry its own git worktree: use it only when two agents edit the same repo at once and their edits would collide, or for a risky experiment that must not touch the working tree. Agents that only read or only produce text take `isolated: false`.
