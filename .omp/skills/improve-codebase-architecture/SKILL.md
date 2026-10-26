---
name: improve-codebase-architecture
description: Scan a codebase for deepening opportunities, present them as a visual HTML report, then grill through whichever one you pick.
disable-model-invocation: true
---

# Improve Codebase Architecture

Surface architectural friction and propose **deepening opportunities**: refactors that turn shallow modules into deep ones, for testability and AI-navigability.

Built on `skill://codebase-design`'s vocabulary — **module**, **interface**, **depth**, **seam**, **adapter**, **leverage**, **locality** — and its principles. Use those terms exactly; never drift into "component," "service," "API," or "boundary." `GLOSSARY.md` names good seams; ADRs in `docs/adr/` are decisions not to re-litigate.

## Process

### 1. Explore

**Scope before you scan: YAGNI.** Deepening eases future changes, so weight recently changed code. Decide *where* to look first:

- User named a direction (module, subsystem, pain point)? Take it; skip the inference.
- Otherwise walk back `git log --oneline` for hot spots: files and areas that keep coming up. Scattered changes, no hot spot → widen the net.

Read `GLOSSARY.md` and area ADRs first.

Then walk the codebase with **one `task` call, one entry, `agent: scout`**: the brief carries the scoped paths, the friction questions and the deletion test (from `skill://improve-codebase-architecture/references/scan-brief.md`), plus `solutionSpace` saying the friction is unknown. It reports friction, not conclusions.

### 2. Present candidates as an HTML report

`write` one self-contained HTML file to the OS temp directory so nothing lands in the repo: `$TMPDIR`, else `/tmp` (`%TEMP%` on Windows), as `<tmpdir>/architecture-review-<timestamp>.html`. Open it (`open`/`xdg-open`/`start`), then **reply with the path and a ≤3-line summary**; never paste it into chat.

One card per candidate — **Files**, **Problem**, **Solution**, **Benefits** (locality, leverage, tests), side-by-side **Before/After** diagram, **Recommendation strength** badge (`Strong` | `Worth exploring` | `Speculative`) — then a **Top recommendation**. read `skill://improve-codebase-architecture/HTML-REPORT.md` for the scaffold, card fields, diagram patterns and styling.

Name the domain from `GLOSSARY.md` and the architecture from `skill://codebase-design`: "the Order intake module," not "the FooBarHandler."

**ADR conflicts**: surface one only when the friction warrants revisiting the ADR, marked in the card (_"contradicts ADR-0007, but worth reopening because…"_). Don't list every refactor an ADR forbids.

No interfaces yet. `ask`: "Which of these would you like to explore?"

### 3. Grilling loop

Once the user picks a candidate, read `skill://grilling` and walk their decision tree: constraints, dependencies, the shape of the deepened module, what sits behind the seam, what tests survive.

Apply `skill://domain-modeling` inline as decisions crystallize: add a newly named concept to `GLOSSARY.md` (create it lazily), sharpen fuzzy terms there, and offer an ADR when a rejection has a load-bearing reason (_"Want me to record this as an ADR so future architecture reviews don't re-suggest it?"_) — only when a future explorer would need it, never for ephemeral or self-evident reasons.

To explore alternative interfaces, use `skill://codebase-design`'s design-it-twice pattern (`skill://codebase-design/DESIGN-IT-TWICE.md`).
