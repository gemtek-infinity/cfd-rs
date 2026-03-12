# Phase 5 Overhaul Status

## Purpose

This document tracks the current execution status of the repository's Big Phase 5
overhaul work.

It does not own phase truth, lane truth, or promotion truth.

For those, the governing files remain:

- `docs/promotion-gates.md`
- `REWRITE_CHARTER.md`
- `STATUS.md`
- `docs/compatibility-scope.md`

For detailed execution planning, `FINAL_PHASE.md` wins.

This file exists to record:

- what Big Phase 5 execution work is active now
- what has already been established
- what remains unfinished
- which stage is currently in progress
- what the next bounded actions are

## Current Position

What exists now:

- accepted first-slice parity-backed config, credentials, and ingress behavior
- a narrow Rust executable surface centered on `validate`, `run`, `help`, and `version`
- a real runtime shell
- a partial transport, protocol, and proxy path
- deployment-proof and runtime-evidence work for the admitted lane
- partial Phase 5.1 wire and stream-serving work
- three live parity ledgers:
  - `docs/parity/cli/implementation-checklist.md`
  - `docs/parity/cdc/implementation-checklist.md`
  - `docs/parity/his/implementation-checklist.md`
- a repository execution plan for the overhaul in `FINAL_PHASE.md`

What does not exist yet:

- full CLI parity to the frozen baseline
- broad Cloudflare contract parity
- broad host and service interaction parity
- complete evidence-backed audit coverage across the three ledgers
- final-phase documentation truth across the repository
- the target crate map as the actual workspace structure
- production-alpha parity proof for the declared lane

## Final-Phase Structure

The overhaul is executed in three mandatory ordered stages:

1. audit
2. reconcile docs
3. refactor

That order is intentional.

We audit first so ownership boundaries are derived from upstream truth rather
than local preference.

We reconcile docs second so contributors are working from honest repository
truth.

We refactor third so the workspace structure follows audited parity surfaces
rather than guesses.

The final phase is organized around three primary parity domains:

- CLI
- CDC
- HIS

### CLI

This domain covers the blackbox command surface, including:

- command tree
- help and usage text
- flag names and aliases
- environment-variable bindings
- hidden and compatibility-only commands
- exit codes
- stdout and stderr placement
- formatting and spacing details

### CDC

This domain covers interactions between cloudflared and Cloudflare-managed
services and contracts, including:

- registration RPC and related registration content
- stream request and response contracts
- management and log-streaming contracts
- readiness and metrics contracts where externally relevant
- Cloudflare API interactions used by tunnel-related commands

### HIS

This domain covers interactions between cloudflared and the local host,
including:

- filesystem effects
- config discovery and file creation
- service and supervision behavior
- diagnostics collection
- watcher and reload behavior
- local endpoint exposure
- environment and privilege assumptions

## Tracking Documents

Primary execution and tracking documents:

- `FINAL_PHASE.md`
- `docs/parity/cli/implementation-checklist.md`
- `docs/parity/cdc/implementation-checklist.md`
- `docs/parity/his/implementation-checklist.md`

Additional feature-group parity documents may be added under `docs/parity/`
when the master ledgers would otherwise become too dense to review effectively.

## Stage Status

### Stage 1: Audit

Status: in progress

Outputs established now:

- CLI implementation checklist exists
- CDC implementation checklist exists
- HIS implementation checklist exists

Remaining audit work:

- expand and normalize checklist rows across the three ledgers
- capture baseline evidence for the highest-risk feature groups
- add feature-group audit documents where needed
- rank major parity gaps for documentation and refactor ordering
- record intentional divergences explicitly when they are found

Immediate focus:

- seed and refine the three ledgers from frozen baseline truth and current Rust reality
- capture CLI blackbox truth
- inventory high-risk CDC contracts
- inventory high-risk HIS contracts

### Stage 2: Reconcile Docs

Status: not started

Required outputs:

- rewritten top-level repository truth
- updated README and docs map
- updated phase and status wording
- updated crate-ownership wording
- target crate README content
- links from top-level docs to parity ledgers

This stage is mandatory implementation work, not cleanup.

### Stage 3: Refactor

Status: not started

Target crate map:

- `cfdrs-bin`
- `cfdrs-cli`
- `cfdrs-cdc`
- `cfdrs-his`
- `cfdrs-shared`

Refactor purpose:

- align ownership with audited parity domains
- reduce mixed responsibility in the current workspace
- make the repository legible to human contributors
- make future parity work land in the right crate by default

Refactor constraint:

- do not create target crates or begin ownership moves before the Stage 1 audit gate
  and the minimum Stage 2 documentation gate described in `FINAL_PHASE.md` are satisfied

## Known High-Risk Areas

The highest-risk areas currently visible are:

- exact CLI surface mismatch
- hidden and compatibility command paths
- registration RPC parity
- actual wire framing and codec parity
- incoming stream round-trip behavior
- management and diagnostics contracts
- readiness semantics
- service installation behavior
- watcher and reload behavior
- filesystem side effects and host layout assumptions

## Anti-Drift Rules

- `docs/promotion-gates.md` owns phase and promotion truth
- `FINAL_PHASE.md` owns overhaul execution detail
- this file records current status only
- do not claim parity from Rust code shape alone
- do not let docs describe intended structure as current reality before it exists
- do not refactor before the owning parity surface is audited
- do not create target crates before the documented audit and documentation gates are satisfied
- do not use the shared crate as a dumping ground
- do not record vague progress such as “mostly done”
- do not leave divergences undocumented

## Progress Reporting Model

Progress should be reported in terms of:

- audited feature groups
- reconciled documents
- completed refactor waves
- parity-backed closures
- named remaining gaps

Avoid reporting progress only in terms of file count or code movement.

## Immediate Next Actions

1. expand the seeded rows in the three implementation checklists from known baseline and current Rust reality
2. capture CLI blackbox truth and attach it to the relevant CLI ledger rows
3. add feature-group audit documents where the ledgers would otherwise become unreadable
4. inventory high-risk CDC contracts with special focus on registration and stream wire semantics
5. inventory high-risk HIS contracts with special focus on service, diagnostics, filesystem, and reload behavior
6. draft the top-level documentation reconciliation map from the first audit findings
7. define refactor migration slices in documents only
