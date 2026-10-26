---
name: codebase-design
description: Shared vocabulary for designing deep modules. Use when the user wants to design or improve a module's interface, find deepening opportunities, decide where a seam goes, make code more testable or AI-navigable, or when another skill needs the deep-module vocabulary.
---

# Codebase Design

Design **deep modules**: a lot of behaviour behind a small interface, at a clean seam, testable through that interface. The aim: leverage for callers, locality for maintainers, testability for everyone.

## Glossary

Use these terms exactly; never substitute "component," "service," "API," or "boundary." Consistent language is the whole point.

- **Module**: anything with an interface and an implementation; scale-agnostic (function, class, package, tier-spanning slice). _Avoid_: unit, component, service.
- **Interface**: everything a caller must know: type signature, plus invariants, ordering constraints, error modes, configuration, performance. _Avoid_: API, signature.
- **Implementation**: the code inside a module. Distinct from **Adapter**: a small adapter can have a large implementation (a Postgres repo). Say "adapter" when the seam is the topic.
- **Depth**: leverage at the interface: behaviour a caller or test exercises per unit of interface learned. **Deep** = much behaviour behind a small interface; **shallow** = interface nearly as complex as the implementation.
- **Seam** _(Michael Feathers)_: a place you can alter behaviour without editing in that place; the location a module's interface lives at, chosen deliberately. _Avoid_: boundary (overloaded with DDD).
- **Adapter**: a concrete thing satisfying an interface at a seam; a role, not a substance.
- **Leverage**: what callers get from depth: more capability per unit of interface learned; one implementation pays back across N call sites and M tests.
- **Locality**: what maintainers get from depth: change, bugs, knowledge, verification concentrate in one place. Fix once, fixed everywhere.

## Principles

- **Depth is a property of the interface, not the implementation.** A deep module may be internally composed of small, mockable, swappable parts; they are not part of the interface. Modules can have **internal seams** (private, used by their own tests) and one **external seam**.
- **The deletion test.** Imagine deleting the module. Complexity vanishes → it was a pass-through. Complexity reappears across N callers → it earned its keep.
- **The interface is the test surface.** Callers and tests cross the same seam; wanting to test *past* the interface means the module is the wrong shape.
- **One adapter = hypothetical seam, two = real.** Don't introduce a seam unless something actually varies across it.

## Going deeper

- Relationships, rejected framings, deep/shallow diagrams, interface-shrinking questions, testable-design recipes: read `skill://codebase-design/references/depth-and-testability.md`.
- Deepening a cluster given its dependencies: read `skill://codebase-design/DEEPENING.md`.
- Exploring alternative interfaces: read `skill://codebase-design/DESIGN-IT-TWICE.md`.
