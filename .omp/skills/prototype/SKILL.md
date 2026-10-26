---
name: prototype
description: Build a throwaway prototype to answer a design question. Use when the user wants to sanity-check whether a state model or logic feels right, or explore what a UI should look like.
---

# Prototype

A prototype is **throwaway code that answers a question**. The question decides the shape.

## Pick a branch

Identify which question is being answered, using the user's prompt, the surrounding code, or `ask` if the user is around:

- **"Does this logic / state model feel right?"** → read `skill://prototype/LOGIC.md`: one shareable HTML file, free-play buttons plus tabbed guided walkthroughs, that a non-developer can drive.
- **"What should this look like?"** → read `skill://prototype/UI.md`: several radically different UI variations on one route, switchable via a URL search param and a floating bottom bar.

Getting this wrong wastes the whole prototype. If the question is genuinely ambiguous and the user isn't reachable, default to whichever branch better matches the surrounding code (a backend module → logic; a page or component → UI) and state the assumption at the top of the prototype.

Either branch ends the same way: **one self-contained artifact file written with `write`**, then **reply with the path plus what to look at** in ≤3 lines — never paste the artifact into chat.

## Rules that apply to both

1. **Throwaway from day one, clearly marked.** Put the prototype next to the module or page it prototypes for, named so a casual reader sees it's a prototype, not production. For throwaway UI routes, obey the project's existing routing convention; don't invent a new top-level structure.
2. **Trivial to run.** A UI prototype starts from one command in the project's task runner (`pnpm <name>`, `python <path>`, `bun <path>`); a logic demo is a single HTML file the user double-clicks.
3. **No persistence by default.** State lives in memory. Persistence is the thing the prototype is _checking_, not something it should depend on. If the question involves a database, hit a scratch DB or a local file clearly named "PROTOTYPE, wipe me".
4. **Skip the polish.** No tests, no error handling beyond what makes the prototype _runnable_, no abstractions. The point is to learn fast.
5. **Surface the state.** After every action (logic) or variant switch (UI), print or render the full relevant state so the user sees what changed.
6. **Capture it when done.** Fold validated decisions into the real code; commit the prototype to a throwaway branch, out of main, with a pointer to that branch on the implementation issue, plus the verdict and the question it settled. Main keeps only the validated decision.
