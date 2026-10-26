---
name: diagnosing-bugs
description: Diagnosis loop for hard bugs and performance regressions. Use when the user says "diagnose"/"debug this", or reports something broken/throwing/failing/slow.
---

# Diagnosing Bugs

Six ordered phases; skip one only when explicitly justified. Read `GLOSSARY.md` (if it exists) and ADRs in the area. Per-phase detail and checklists: `skill://diagnosing-bugs/references/phases.md`.

## Redact

**Redact every secret first**: `<REDACTED>`. Build loops against env vars, so the credential stays in the environment, not in what you show. Captured artifacts carry auth headers: quote only the signal-bearing lines. If that isn't enough to diagnose the bug, say so and ask the user.

## 1 · Build a feedback loop

**This is the skill.** A **tight** pass/fail signal that goes red on _this_ bug finds the cause; everything else consumes it. **Be aggressive, be creative, refuse to give up.** Constructions in preference order, the tightening checklist and the non-deterministic playbook: read `skill://diagnosing-bugs/references/feedback-loops.md`.

Loops run on `bash` (repro commands, harness scripts), `eval` (instrumentation, probes) and `browser` (headless UI loops); human-only steps go through `skill://diagnosing-bugs/scripts/hitl-loop.template.sh`, whose captured output feeds back to you.

**Done when** one command has **already run at least once** (show it and its redacted output) and is red-capable (asserts the **user's exact symptom**, not "runs without erroring"), deterministic (flaky bugs: pinned, high reproduction rate), fast (seconds) and agent-runnable (a human only via the HITL script). A theory before that command exists is the failure this skill prevents: **no red-capable command, no Phase 2.** Unbuildable for real? Stop, list what you tried, and ask for a reproducing environment, a redacted captured artifact, or permission to instrument.

## The remaining gates, in order

- **2 · Reproduce + minimise** — loop red, failing the way the **user** described; shrink to the **smallest scenario that still goes red** until **every remaining element is load-bearing**. No proceeding until reproduced **and** minimised.
- **3 · Hypothesise** — **3–5 ranked, falsifiable** hypotheses, each stating its prediction, before testing any; no prediction = a vibe, discard it. Show the ranked list first; don't block if they're AFK.
- **4 · Instrument** — one variable at a time, each probe mapping to one prediction; debugger/`xd://lsp` before logs, logs tagged `[DEBUG-a4f2]` so cleanup is one `grep`; perf regressions: baseline, then bisect.
- **5 · Fix + regression test** — test **before the fix**, only at a **correct seam** (the real bug pattern at the call site); a shallow seam gives false confidence, and **no correct seam is itself the finding**. Then failing test → fix → pass → re-run the phase 1 loop.
- **6 · Cleanup** — repro gone; test passes (or missing seam documented); `[DEBUG-...]` tags removed; throwaway prototypes deleted; correct hypothesis in the commit / PR message.
