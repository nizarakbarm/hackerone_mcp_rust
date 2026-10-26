# Routing table

Every route outside the main flow, for `skill://ask-matt`. Read the entry's SKILL.md before you act on it.

## On-ramps

A starting situation that generates work, then merges onto the main flow.

- **Bugs and requests piling up** → **`skill://triage`**. It moves issues through triage roles and produces agent-ready issues, which **`skill://implement`** later picks up.

  Triage is only for issues **you didn't create**: bug reports, incoming feature requests, anything that arrives raw. Tickets that `skill://to-tickets` produced are already agent-ready, so **don't triage them**.

- **Something's broken** → **`skill://diagnosing-bugs`**. For the hard ones: the bug that resists a first glance, the intermittent flake, the regression that crept in between two known-good states. It refuses to theorise until it has a **tight feedback loop** (one command that already goes red on *this* bug), then fixes with a regression test. Once the fix is in, run **`skill://retro`** in the same session to ask what would have prevented the bug; where the real finding is that there's no good seam to lock it down, that's a job for **`skill://improve-codebase-architecture`**.

- **A huge, foggy effort: a greenfield project or a huge feature build, too big for one session** → **`skill://wayfinder`**, the most cognitively demanding flow here. When the way from here to the destination isn't visible yet, it charts a **shared map** of **decision tickets** on the issue tracker and resolves them one at a time, producing **decisions, not deliverables**, until the fog is pushed back and the way is clear. Where **`skill://grill-with-docs`** sharpens an idea you can hold in one session, wayfinder is for the idea you can't, and it's slower and denser, so save it for exactly that, never a well-scoped feature.

  When the map clears, **it hands off, it doesn't build**: merge onto the main flow at **`skill://to-spec`**, which collapses the map's linked decisions into a buildable plan, then `skill://to-tickets` and `skill://implement` as usual. Looping the map straight into `skill://implement` skips that collapse and throws the linked detail away, so go straight to `skill://implement` only when the effort turned out genuinely small.

## Codebase health

Not feature work, just upkeep.

- **`skill://improve-codebase-architecture`** runs whenever you have a spare moment to keep the codebase good for agents to operate in. It surfaces **deepening opportunities**; picking one _generates an idea_ you can take into the main flow at `skill://grill-with-docs`. It's the survey that finds the candidates; **`skill://codebase-design`** is the bench you design the chosen one on.

## Vocabulary underneath

Two model-invoked references that run *beneath* the other skills, each the single source of truth for its vocabulary. Reach for them directly when the **words**, not the process, are the problem; or let the skills above pull them in.

- **`skill://domain-modeling`**: sharpen the project's *domain* language: challenge a fuzzy term, resolve an overloaded word ("account" doing three jobs), record a hard-to-reverse decision as an ADR. It's the active discipline `skill://grill-with-docs` drives to keep `GLOSSARY.md` a clean glossary.
- **`skill://codebase-design`** is the deep-module vocabulary (module, interface, depth, seam, adapter, leverage, locality) for designing a module's *shape*: a lot of behaviour behind a small interface at a clean seam. `skill://tdd` and `skill://improve-codebase-architecture` both speak it.

## Standalone

Off the main flow entirely.

- **`skill://grill-me`**: the same relentless interview as `skill://grill-with-docs`, but **stateless**: it saves nothing locally and builds no `GLOSSARY.md`. Reach for it when you are **not working in a working directory** (sharpening a plan, a design, a piece of writing, anything with no repo under it). If you are in a working directory, use `skill://grill-with-docs` instead: it runs the same interview and leaves a paper trail, so it is strictly the better one.
- **`skill://grilling`** is the interview primitive itself: rounds, the frontier, facts are the agent's job and decisions are yours. `skill://grill-me` and `skill://grill-with-docs` are the two named ways in, and `skill://triage`, `skill://wayfinder` and `skill://improve-codebase-architecture` all run it internally. Reach for it directly only when you want the interview with no wrapper around it.
- **`skill://prototype`** is a small, throwaway program that answers one design question: does this state model feel right, or what should this UI look like. Throwaway is a constraint on how the code is written, not a promise to destroy it: the answer folds into the real code, and the prototype itself is kept as a **primary source** on a `prototype/<name>` branch out of main, pointed at from the implementation issue. It's the detour in step 2 of the main flow, but reach for it any time a design question is hard to settle on paper.
- **`skill://research`**: delegate reading legwork to a **background agent**: it investigates a question against **primary sources**, then leaves a cited Markdown file in the repo. Keep working while it reads. The file it produces is something to take *into* the main flow at `skill://grill-with-docs`, since research feeds the thinking rather than replacing it.
- **`skill://wizard`** is for the steps only a **human** can take: provisioning infrastructure, setting up credentials or CI secrets, clicking through an unfamiliar third-party dashboard, running a one-off migration or cutover. It generates an interactive bash script that opens each URL, captures each value, and writes it into `.env` and GitHub secrets, so the procedure stops being something you re-explain to an agent every time. Model-invoked, so the agent reaches for it the moment it hits a wall only you can pass. If the agent could just do it itself, it should; this is for where a human is genuinely in the loop.
- **`skill://writing-for-agents`** is the reference for writing documents agents consume: skills, `AGENTS.md`, pointed-at docs.

The upstream skill set also has `teach` (learn a concept over multiple sessions, using the current directory as a stateful workspace), `to-questionnaire` (interview you about the *send* and write a questionnaire for whoever holds the blocker), and `wait-what` (re-pitch a message that didn't land, in plain English and `GLOSSARY.md` vocabulary). None of the three is **installed in this repo**, so nothing here routes to them: if that is the need you actually have, say so rather than reaching for a skill that isn't there.

## Precondition

**`skill://setup-matt-pocock-skills`**: run before your first engineering flow to configure the issue tracker, triage labels, and doc layout the other skills assume. Custom issue trackers also work.
