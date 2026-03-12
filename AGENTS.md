# AGENTS.md

This file is the short operating guide for coding agents in this repository.

Keep it short.
Do not turn this file into a status report, architecture dump, dependency catalog, or command manual.

Start cold reads with `docs/ai-context-routing.md`.
Use this file as the short operating guide, not as the full routing map.

## Use the right file

- `docs/ai-context-routing.md`
  - minimum-file routing for cold starts
  - staged retrieval order

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

- `docs/adr/ADR-0006-standard-format-and-workspace-dependency-admission.md`
  - standard-format and workspace-dependency admission policy

- `SKILLS.md`
  - repeatable porting workflow

- `FINAL_PLAN.md`
  - staged execution plan for the final phase

- `FINAL_PHASE.md`
  - detailed execution reference

- `docs/parity/README.md`
  - parity domain index and document map

- `docs/parity/cli/implementation-checklist.md`
  - CLI parity ledger

- `docs/parity/cdc/implementation-checklist.md`
  - CDC parity ledger

- `docs/parity/his/implementation-checklist.md`
  - HIS parity ledger

- `docs/deployment-notes.md`
  - operator deployment contract and known gaps

- `CONTRIBUTING.md`
  - human contributor guide
  - build and test instructions
  - code style and engineering standards pointers
  - parity evidence workflow

- `docs/code-style.md`
  - code style reference (28 rules with quick-reference summary)

- `docs/engineering-standards.md`
  - engineering standards reference (7 standards with quick-reference summary)

## Working rules

- do not treat this repository as a blank-slate Rust project
- do not edit frozen inputs during normal rewrite work
- do not claim parity from Rust code alone
- do not silently widen scope
- do not preload speculative dependencies
- prefer synchronous and deterministic code unless the accepted slice requires async
- keep patches narrow and source-grounded
- when finishing Rust work, follow the completion workflow in `.github/instructions/rust.instructions.md` (test+clippy → debtmap gate → fmt → summary+docs)
- for parity work, identify the domain (CLI, CDC, or HIS) and update the relevant ledger

## Question routing

Use `docs/ai-context-routing.md` for the detailed task-to-file map.

For repo-state, active-phase, scope-lane, runtime-deps, lane-decisions, behavior-baseline, and governing-files questions, prefer the local MCP snapshot surface first before loading larger docs or frozen trees.

If evidence is missing or conflicting, say so explicitly.
