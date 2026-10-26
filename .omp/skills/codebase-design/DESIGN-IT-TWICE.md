# Design It Twice

When the user wants to explore alternative interfaces for a chosen deepening candidate, use this parallel sub-agent pattern. Based on "Design It Twice" (Ousterhout): your first idea is unlikely to be the best.

Uses the vocabulary in `skill://codebase-design`: **module**, **interface**, **seam**, **adapter**, **leverage**.

## Process

### 1. Frame the problem space

Before spawning sub-agents, write a user-facing explanation of the problem space for the chosen candidate:

- The constraints any new interface would need to satisfy
- The dependencies it would rely on, and which category they fall into (see `skill://codebase-design/DEEPENING.md`)
- A rough illustrative code sketch to ground the constraints, not a proposal, just a way to make the constraints concrete

Show this to the user, then immediately proceed to Step 2. The user reads and thinks while the sub-agents work in parallel.

### 2. Spawn the designers

Make **one `task` call with three entries** (`agent: task`, `isolated: false` — designers only write briefs, so they share the working tree and need no worktree), all in parallel. Fan out at most 4; three is the default, add a fourth only when a candidate has a cross-seam dependency worth a ports-and-adapters design.

Each entry must produce a **radically different** interface for the deepened module. The `task` brief carries everything: children start blank, so include the technical facts yourself (file paths, coupling details, dependency category from `skill://codebase-design/DEEPENING.md`, what sits behind the seam). The brief is independent of the user-facing problem-space explanation in Step 1. Give each entry a different design constraint:

- Entry 1: "Minimize the interface: aim for 1–3 entry points max. Maximise leverage per entry point."
- Entry 2: "Maximise flexibility: support many use cases and extension."
- Entry 3: "Optimise for the most common caller: make the default case trivial."
- Entry 4 (only if applicable): "Design around ports & adapters for cross-seam dependencies."

Include both `skill://codebase-design` vocabulary and the project's `GLOSSARY.md` vocabulary in the brief so each designer names things consistently with the architecture language and the domain language. Set `solutionSpace` to say the design is genuinely open: several interface shapes are viable and the trade-offs are theirs to judge.

Each designer outputs:

1. Interface (types, methods, params, plus invariants, ordering, error modes)
2. Usage example showing how callers use it
3. What the implementation hides behind the seam
4. Dependency strategy and adapters (see `skill://codebase-design/DEEPENING.md`)
5. Trade-offs: where leverage is high, where it's thin

### 3. Present and compare

Present designs sequentially so the user can absorb each one (one message each, or one file per design plus its path and a three-line summary), then compare them in prose. Contrast by **depth** (leverage at the interface), **locality** (where change concentrates), and **seam placement**.

After comparing, give your own recommendation: which design you think is strongest and why. If elements from different designs would combine well, propose a hybrid. Be opinionated: the user wants a strong read, not a menu.
