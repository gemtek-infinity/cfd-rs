# Documentation Map

This directory holds repository policy, scope, status, and parity documents.
Use this directory as an index. Load the smallest relevant file first.

## Start Here

- `docs/ai-context-routing.md` — minimum-file routing for cold starts
- `REWRITE_CHARTER.md` — non-negotiables, active lane, scope
- `STATUS.md` — short current-state index
- `docs/promotion-gates.md` — phase model and promotion boundaries

## Policy

- `docs/compatibility-scope.md` — what "compatible" means
- `docs/build-artifact-policy.md` — build and artifact policy
- `docs/dependency-policy.md` — dependency admission, workspace dependency truth
- `docs/allocator-runtime-baseline.md` — allocator and runtime baseline
- `docs/go-rust-semantic-mapping.md` — concurrency and lifecycle doctrine

## Current State

- `docs/status/rewrite-foundation.md` — baseline, workspace shape, source precedence
- `docs/status/active-surface.md` — current crate content and absent surfaces
- `docs/status/phase-5-overhaul.md` — Big Phase 5 execution tracker

### Historical (first-slice era)

- `docs/status/first-slice-parity.md` — first-slice closure record
- `docs/status/porting-rules.md` — first-slice porting rules
- `docs/first-slice-freeze.md` — first-slice freeze record

## Parity Audit And Tracking

Parity navigation index: `docs/parity/README.md`

Implementation checklists (live parity ledgers):

- `docs/parity/cli/implementation-checklist.md` — CLI (32 rows)
- `docs/parity/cdc/implementation-checklist.md` — CDC (44 rows)
- `docs/parity/his/implementation-checklist.md` — HIS (74 rows)

Feature-group audit documents:

- `docs/parity/cli/root-and-global-flags.md`
- `docs/parity/cli/tunnel-subtree.md`
- `docs/parity/cli/access-subtree.md`
- `docs/parity/cli/tail-and-management.md`
- `docs/parity/cdc/registration-rpc.md`
- `docs/parity/cdc/stream-contracts.md`
- `docs/parity/cdc/management-and-diagnostics.md`
- `docs/parity/cdc/metrics-readiness-and-api.md`
- `docs/parity/his/service-installation.md`
- `docs/parity/his/filesystem-and-layout.md`
- `docs/parity/his/diagnostics-and-collection.md`
- `docs/parity/his/reload-and-watcher.md`

Baseline evidence captures: `docs/parity/cli/captures/`

## Overhaul Execution

- `FINAL_PLAN.md` — staged execution plan with sub-stage gates
- `FINAL_PHASE.md` — detailed execution reference

## Operator Guidance

- `docs/deployment-notes.md` — deployment contract, build-to-run flow, known gaps

## ADRs

- `docs/adr/` — architecture decision records; load the smallest relevant one

## Rust Coding References

- `.github/instructions/rust.instructions.md` — AI and human Rust editing rules
- `docs/code-style.md` — human-facing code style reference
- `docs/engineering-standards.md` — engineering standards reference
