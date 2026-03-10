# Engineering Standards

This is a human-facing reference document.
For default AI code-edit guidance, start with `.github/instructions/rust.instructions.md` and load this file only when deeper explanation is useful.

## Purpose

This document defines how Rust code in this repository should be **structured and owned**.

It is the authority for:

- crate boundaries
- module decomposition
- dependency admission and containment
- abstraction thresholds
- design-pattern usage
- runtime and lifecycle ownership
- public API discipline

This is an **engineering structure** document, not a local code appearance document.
For naming, spacing, comments, control flow, tests, and other local readability rules, see `docs/code-style.md`.

---

## 1. Standard: One owner per responsibility boundary

Each crate and module should have one primary responsibility.

Prefer:

- crates with one clear area of ownership
- modules with one main reason to change
- names that reflect responsibility directly

Avoid:

- mixing unrelated responsibilities in one crate or module
- broad "common" or "misc" layers that become dependency magnets
- placing code together only because it is technically convenient

Review signal:

- if a module needs many unrelated imports, many unrelated tests, or many different kinds of changes, it may own too much

---

## 2. Standard: Public surfaces stay smaller than internals

Public APIs should be narrow, intentional, and harder to misuse than the internals that support them.

Prefer:

- small public types
- small public traits
- private helpers by default
- thin owned facades over implementation detail

Avoid:

- making items public "just in case"
- exposing internal helpers as part of the crate contract too early
- wide public module trees with weak ownership boundaries

Review signal:

- if another crate can touch too much of an implementation directly, the public surface is too wide

---

## 3. Standard: Dependencies enter through owned seams

External crates should enter the repository through clear local boundaries.

Prefer:

- thin adapters around third-party crates when they reduce spread
- local wrapper types where they improve ownership clarity
- shared dependencies declared in `[workspace.dependencies]` when they are intentionally shared

Avoid:

- scattering direct third-party APIs across unrelated crates
- letting external type shapes define unrelated internal APIs
- per-crate dependency drift without an explicit reason

Review signal:

- if changing or replacing a dependency would require touching many unrelated crates, the seam is too wide

---

## 4. Standard: Add abstraction only after a real need exists

This repository prefers concrete code first, abstraction second.

Prefer:

- concrete structs, enums, and functions for the first real path
- abstraction after a second real use case or a clear boundary need
- explicit duplication over premature indirection when the shape is still unstable

Avoid:

- speculative traits
- generic wrappers with only one real implementation
- adding indirection because it merely looks more architectural

Review signal:

- if an abstraction exists, its current benefit should be obvious without needing a future-looking argument

---

## 5. Standard: Encode important invariants in types, not every invariant

Use Rust's type system where it clearly removes invalid states or reduces meaningful misuse.

Prefer:

- newtypes for real semantic distinctions
- enums for closed state sets
- builders when construction is genuinely wide or staged
- typestate when lifecycle misuse is a real risk and the benefit is clear

Avoid:

- type-level cleverness for style points
- typestate for trivial flows
- generic or phantom-type machinery that is harder to understand than the invariant it protects

Review signal:

- if the type machinery is more complex than the bug class it prevents, it is probably too much

---

## 6. Standard: Runtime and lifecycle ownership must be explicit

Long-lived runtime behavior must have an obvious owner.

Prefer:

- explicit ownership of startup, shutdown, reload, and supervision paths
- clearly named runtime or lifecycle modules
- visible handoff points between config, runtime, transport, and proxy layers

Avoid:

- hidden background work
- lifecycle behavior spread across unrelated modules
- "spawn and hope" patterns with unclear ownership

Review signal:

- if shutdown, reload, or task ownership is hard to trace, lifecycle ownership is too diffuse

---

## 7. Standard: Module decomposition should follow reviewable reasoning units

Modules should be sized and split for understanding, not just compilation.

Prefer:

- modules that a reviewer can understand in one focused sitting
- top-level flow that stays visible
- submodules for detail, boundary-specific logic, or format-specific logic when that improves navigation

Avoid:

- giant files that collect everything about a feature
- decomposition based only on line count
- fragmentation into tiny files that add movement without adding clarity

Review signal:

- if a reviewer must scroll repeatedly just to recover the main flow, decomposition is probably too weak

---

## Quick rule of thumb

When choosing between two valid designs, prefer the one that:

1. has clearer ownership
2. has a narrower public surface
3. contains third-party details more effectively
4. is easier to review in small slices
5. keeps runtime and boundary behavior more explicit

Smaller, clearer, more owned boundaries win.
