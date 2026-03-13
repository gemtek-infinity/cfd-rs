# Engineering Standards

This is a reference document for both human contributors and AI agents.
For default AI code-edit guidance, start with [.github/instructions/rust.instructions.md](../.github/instructions/rust.instructions.md) and load this file only when deeper explanation is useful.

## Quick reference

This table summarizes all 13 standards. Each links to its detailed section below.

| # | Standard | One-line summary |
| --- | --- | --- |
| 1 | [One owner per boundary](#1-standard-one-owner-per-responsibility-boundary) | Each crate and module has one primary responsibility |
| 2 | [Narrow public surfaces](#2-standard-public-surfaces-stay-smaller-than-internals) | Public APIs are narrow, intentional, and hard to misuse |
| 3 | [Dependencies through seams](#3-standard-dependencies-enter-through-owned-seams) | External crates enter through clear local boundaries |
| 4 | [Abstraction after need](#4-standard-add-abstraction-only-after-a-real-need-exists) | Concrete code first, abstraction after a second real use |
| 5 | [Types encode invariants](#5-standard-encode-important-invariants-in-types-not-every-invariant) | Newtypes and enums for real distinctions, not type-level cleverness |
| 6 | [Explicit lifecycle ownership](#6-standard-runtime-and-lifecycle-ownership-must-be-explicit) | Startup, shutdown, reload, and supervision have obvious owners |
| 7 | [Reviewable modules](#7-standard-module-decomposition-should-follow-reviewable-reasoning-units) | Modules sized for understanding, not just compilation |
| 8 | [Smaller focused files](#8-standard-prefer-smaller-focused-files-with-clear-ownership) | Clear intent and ownership per file and module |
| 9 | [Use mature crates](#9-standard-use-the-mature-crate-ecosystem-over-reinvention) | Prefer production-ready crates.io dependencies over handwritten alternatives |
| 10 | [Wrapper types with tests](#10-standard-contain-external-dependencies-through-wrapper-types-with-tests) | Wrap and test external crate behavior at local boundaries |
| 11 | [Stack-allocated types](#11-standard-prefer-stack-allocated-types-where-safe) | Default to stack allocation for bounded, predictable sizes |
| 12 | [Zero-copy types](#12-standard-prefer-zero-copy-types-where-practical) | Borrow instead of clone when lifetimes allow |
| 13 | [Async task ownership](#13-standard-long-lived-async-tasks-require-explicit-ownership-and-recovery) | Every spawned task needs an owner, budget, and recovery plan |

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
For naming, spacing, comments, control flow, tests, and other local readability rules, see [docs/code-style.md](code-style.md).

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
- standard-library and mature-crate domain types such as `Uuid`, `SocketAddr`,
  `IpAddr`, and validated byte newtypes before falling back to raw `String` or
  `Vec<u8>` storage
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

## 8. Standard: Prefer smaller focused files with clear ownership

Each file and module should have a clear intent, ownership, and responsibility.

Prefer:

- files with one focused area of behavior
- modules with a clear, nameable purpose
- breaking large modules into smaller submodules when distinct ownership
  boundaries emerge
- keeping test modules inside the file — test code does not count as an
  enlarging factor

Avoid:

- files that grow because new behavior gets appended without questioning fit
- modules where different readers need different mental models to follow
- broad "everything about X" files when X naturally splits into distinct
  ownerships

Review signal:

- if a file requires context-switching between unrelated concerns while
  reading, it should be split
- if you cannot describe the file's purpose in one phrase, it owns too much

---

## 9. Standard: Use the mature crate ecosystem over reinvention

Do not reinvent functionality when a mature, production-ready crate exists on
crates.io.

Prefer:

- mature, production-ready, actively maintained crates with strong community
  adoption
- crates with security audit history or trusted maintainers for
  security-sensitive paths
- high-performance crates that are well-benchmarked in production environments
- common ecosystem crates that reduce learning overhead for new contributors

Avoid:

- handwriting parsing, encoding, or validation logic that mature crates handle
  better
- choosing niche or unmaintained crates over established ones
- adding dependencies for trivial operations that `std` handles adequately
- using crates that are experimental, pre-1.0 without strong production
  evidence, or poorly documented

This standard complements [docs/dependency-policy.md](dependency-policy.md), which governs admission
mechanics. This standard governs the design decision of build-vs-reuse.

Review signal:

- if handwritten code replicates what a well-known crate does better, prefer
  the crate
- if a crate has fewer than a few hundred downloads or has been abandoned,
  prefer alternatives

---

## 10. Standard: Contain external dependencies through wrapper types with tests

When a third-party crate's API surface could leak into unrelated modules, wrap
it.

Prefer:

- thin wrapper structs or enums around external crate types when they control
  behavior or limit API surface
- wrapper types that express repository-local intent rather than re-exporting
  the full external API
- unit tests for wrapper behavior, especially when the wrapper normalizes,
  restricts, or extends external behavior

Avoid:

- wrappers for trivial operations where direct use is clear and contained
- wrappers that add no behavioral value beyond indirection
- untested wrapper types — if you write a wrapper, test it

This reinforces Standard 3 (dependencies through seams) with the additional
requirement that behavioral wrappers carry tests.

Review signal:

- if a wrapper exists but has no tests, it is incomplete
- if removing a wrapper would scatter the external API, the wrapper has value

---

## 11. Standard: Prefer stack-allocated types where safe

Default to stack allocation when the size is bounded and predictable.

Prefer:

- fixed-size arrays, tuples, and small structs on the stack
- `ArrayVec`, `SmallVec`, or similar stack-first containers when the maximum
  size is bounded and small
- zero-allocation parsing paths when the target type has a fixed or bounded
  representation

Avoid:

- heap allocation for data that fits comfortably on the stack
- deeply recursive or unbounded stack allocation that risks stack overflow
- assuming stack is always safe — inspect and predict the calling stack depth
  for recursive paths

Review signal:

- if a hot path allocates heap memory for a small bounded value, prefer stack
  allocation
- if a recursive function carries stack-heavy locals, verify the expected call
  depth

---

## 12. Standard: Prefer zero-copy types where practical

Avoid unnecessary copies when the data lifetime allows borrowing.

Prefer:

- `&str` and `&[u8]` over owned `String` and `Vec<u8>` when the owner outlives
  the consumer
- `Cow<'_, str>` and `Cow<'_, [u8]>` when a value is borrowed in most paths
  but occasionally needs ownership
- zero-copy deserialization crates and patterns when parsing performance matters
- `bytes::Bytes` for reference-counted shared immutable byte buffers in I/O
  paths

Avoid:

- cloning data at API boundaries when a borrow would suffice
- overusing `'static` lifetimes that force unnecessary ownership transfer
- premature zero-copy optimization when the path is not performance-sensitive

Review signal:

- if a function clones its input only to return a value derived from it,
  consider borrowing
- if a hot path allocates and copies where a slice reference is sufficient,
  prefer the reference

---

## 13. Standard: Long-lived async tasks require explicit ownership and recovery

When using long-lived `tokio::spawn` as an event loop or background worker,
every task must have an explicit owner, resource budget, and recovery strategy.

Prefer:

- explicit ownership — every spawned task has a clear parent or supervisor
- bounded resource utilization — explicit limits on memory, connections, and
  queue depth
- recovery strategy — define what happens when the task panics, errors, or
  stalls
- cancellation propagation — use `CancellationToken` for hierarchical shutdown
- performance justification — the task must justify its resource cost versus
  alternatives

Avoid:

- fire-and-forget spawns without join handles or ownership tracking
- spawning long-lived tasks for work that could be done synchronously or
  on-demand
- hidden resource growth — tasks that silently accumulate memory or connections
- spawn-and-hope patterns where failure detection is accidental

This standard adopts the supervisor ownership model from
[docs/go-rust-semantic-mapping.md](go-rust-semantic-mapping.md). See that document for the recommended
shapes (bounded mpsc, oneshot replies, CancellationToken trees, JoinSet
ownership).

Review signal:

- if a spawned task has no visible owner, supervisor, or shutdown path, it
  violates this standard
- if a long-lived task has no bounded resource budget, it risks silent growth

---

## Quick rule of thumb

When choosing between two valid designs, prefer the one that:

1. has clearer ownership
2. has a narrower public surface
3. contains third-party details more effectively
4. is easier to review in small slices
5. keeps runtime and boundary behavior more explicit
6. reuses mature ecosystem crates over handwritten logic
7. prefers stack and zero-copy when the data shape allows
8. gives every long-lived task an explicit owner and shutdown path

Smaller, clearer, more owned boundaries win.
