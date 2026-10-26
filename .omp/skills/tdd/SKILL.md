---
name: tdd
description: Test-driven development. Use when the user wants to build features or fix bugs test-first, mentions "red-green-refactor", or wants integration tests.
---

# Test-Driven Development

TDD is the red → green loop. Consult every section here during each cycle, not after.

Read `GLOSSARY.md` (if it exists) so test names and interface vocabulary match the project's domain language, and respect ADRs in the area you're touching.

## What a good test is

Tests verify behavior through public interfaces, not implementation details. A good test reads like a specification ("user can checkout with valid cart") and survives refactors because it doesn't care about internal structure.

Read `skill://tdd/tests.md` for good/bad examples and `skill://tdd/mocking.md` for when and how to mock.

## Seams: where tests go

A **seam** is the public boundary you test at: the interface where you observe behavior without reaching inside. Tests live at seams, never against internals.

**Test only at pre-agreed seams.** Before writing any test, write down the seams under test and confirm them with the user (`ask` if the choice is genuinely open); no test is written at an unconfirmed seam. Agreeing them up front is how testing effort lands on the critical paths and complex logic instead of every edge case.

Ask: "What's the public interface, and which seams should we test?" Give each proposed seam a one-line note on what it catches and what it misses.

When the interface's own shape is in question (how deep the module is, where the seam belongs, what it should expose), read `skill://codebase-design` for the shared vocabulary: module, interface, depth, seam, adapter, leverage, locality.

## Anti-patterns

- **Implementation-coupled**: mocks internal collaborators, tests private methods, or verifies through a side channel (querying the database instead of using the interface). Tell: it breaks when you refactor but behavior hasn't changed.
- **Tautological**: the assertion recomputes the expected value the way the code does, so it passes by construction. Expected values come from an independent source of truth: a known-good literal, a worked example, the spec.
- **Horizontal slicing**: all tests first, then all implementation; that verifies _imagined_ behavior and locks in test structure before you understand the implementation. Work in **vertical slices**: one test → one implementation → repeat, each test a **tracer bullet** responding to what the last cycle taught you.

## Rules of the loop

- **Red before green.** Write the failing test first, then only enough code to pass it. Don't anticipate future tests or add speculative features.
- **One slice at a time.** One seam, one test, one minimal implementation per cycle.
- **Refactoring is not part of the loop.** It belongs to the review stage (read `skill://code-review`), not the red → green implementation cycle.
