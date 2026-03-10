# AGENTS.md

This file is the short operating guide for coding agents in this repository.

Keep it short.
Do not turn this file into a status report, architecture dump, dependency catalog, or command manual.

## Use the right file

- `REWRITE_CHARTER.md`
  - shortest non-negotiables
  - active lane
  - scope boundary

- `STATUS.md`
  - what exists now
  - what is partial
  - what is still unported

- `docs/compatibility-scope.md`
  - what "compatible" means

- `docs/dependency-policy.md`
  - dependency admission rules

- `docs/allocator-runtime-baseline.md`
  - allocator and runtime admission rules

- `docs/go-rust-semantic-mapping.md`
  - concurrency and lifecycle doctrine

- `docs/adr/0001-hybrid-concurrency-model.md`
  - ADR-level runtime decision

- `SKILLS.md`
  - repeatable porting workflow

## Working rules

- do not treat this repository as a blank-slate Rust project
- do not edit frozen inputs during normal rewrite work
- do not claim parity from Rust code alone
- do not silently widen scope
- do not preload speculative dependencies
- do not introduce async/runtime machinery into first-slice work unless the accepted slice requires it
- keep patches narrow and source-grounded

## Question routing

Before answering or patching, classify the task:

1. behavior / parity
   - use Go code/tests first
   - use design-audit second

2. current repository state
   - use `STATUS.md`

3. scope / lane / non-negotiables
   - use `REWRITE_CHARTER.md`

4. dependency / allocator / runtime policy
   - use the matching file under `docs/`

If evidence is missing or conflicting, say so explicitly.
