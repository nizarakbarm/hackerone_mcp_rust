---
name: pr
description: "Use when writing a PR body."
---

# PR

Use this template for the PR body:

```markdown
## Summary

<diagram, diff-sketch, or tree>

## Evidence

- **Before:** <screenshot/output/failing test run>
  **After:** <screenshot/output/passing test run>

## Merge Danger

**Door:** <one-way or two-way>

<optional: description>

**Blast Radius:** <one-word description>

<optional: potential ramifications of merge>
``

Skip all preambles and keep prose brief. Use the user's domain language from `GLOSSARY.md`. If you draft the body as a file, reply with its path plus a ≤3-line summary rather than pasting it back.

## Sections

### Summary

Pick the smallest view that makes the key point clear. The menu: pseudocode (logic/algorithm), a call tree (runtime control flow), a component tree with the state and module boundaries that matter (UI structure), a shallow file tree (file responsibility, broad refactors), Mermaid (component interaction, control flow, data flow), `diff` (when the point is what changes and the surrounding shape already exists), or the whole block (most of it is new, or omitted context would hide ownership or order). Full examples of each, including the diff shapes for component, file-layout, call-tree, and state/control-flow changes: read `skill://pr/references/summary-visuals.md`.

#### Guidance

Place each visual next to the short text it supports. Keep only the calls, files, props, states, and boundaries needed to answer the user's current question or the options to resolve the current discussion point. You may use one of these, several, or (unlikely) all of them. Use your judgement and don't overwhelm the user.

### Evidence

Concrete evidence that the change works. Show a before and after. Screenshots are S-tier — when the environment is set up for it and the change is visual. Execution-based evidence is A-tier: test results, console output. Show the exact test that now fails and passes, using pseudocode.

### Merge Danger

Describe whether it's a one-way or two-way door. You can walk back through two-way doors, but not one-way doors. A PR that is cheap to roll back is lower risk. Changes that involve destructive actions or hard-to-reverse decisions are one-way doors.

The blast radius is the potential impact or scope of the changes introduced by this PR. Consider all possibilities. Examples are layout shift, breakages for consumers, mobile responsiveness, etc.
