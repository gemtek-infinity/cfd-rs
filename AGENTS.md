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

- `docs/build-artifact-policy.md`
  - local dev build expectations
  - CI validation policy
  - shipped artifact policy

- `docs/promotion-gates.md`
  - current big-phase model
  - active phase/task
  - promotion boundaries

- `docs/dependency-policy.md`
  - dependency admission rules

- `docs/allocator-runtime-baseline.md`
  - allocator and runtime admission rules

- `docs/go-rust-semantic-mapping.md`
  - concurrency and lifecycle doctrine

- `docs/adr/0001-hybrid-concurrency-model.md`
  - ADR-level runtime decision

- `docs/adr/0002-transport-tls-crypto-lane.md`
  - transport / TLS / crypto lane decision

- `docs/adr/0003-pingora-critical-path.md`
  - Pingora critical-path scope decision

- `docs/adr/0004-fips-in-alpha-definition.md`
  - FIPS-in-alpha boundary and validation definition

- `docs/adr/0005-deployment-contract.md`
  - Linux deployment contract definition

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

1. behavior / parity: use Go code/tests first, then design-audit
2. current repository state: use `STATUS.md`
3. phase model / promotion boundaries: use `docs/promotion-gates.md`
4. scope / lane / non-negotiables: use `REWRITE_CHARTER.md`
5. build / artifact policy: use `docs/build-artifact-policy.md`
6. transport / TLS / crypto lane: use `docs/adr/0002-transport-tls-crypto-lane.md`
7. Pingora critical path: use `docs/adr/0003-pingora-critical-path.md`
8. FIPS-in-alpha boundary: use `docs/adr/0004-fips-in-alpha-definition.md`
9. deployment contract: use `docs/adr/0005-deployment-contract.md`
10. dependency / allocator / runtime policy: use the matching file under `docs/`

If evidence is missing or conflicting, say so explicitly.
