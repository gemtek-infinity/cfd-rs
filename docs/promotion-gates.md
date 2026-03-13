# Promotion Gates

This document defines the current promotion model for the rewrite.
It stays compact on purpose: `STATUS.md` owns tracked status, and
`docs/phase-5/roadmap.md` owns the implementation roadmap.

## Purpose

Promotion happens by current evidence, not by intent.

Use this file to answer three questions only:

- which big phase is active
- which Phase 5 milestone is active
- what evidence is required before the next promotion claim

## Current Model

Closed and frozen big phases:

- Big Phase 1 — baseline truth and shared-behavior compare were frozen
- Big Phase 2 — Linux production-alpha lane, dependency policy, and ADR lane decisions were frozen
- Big Phase 3 — minimum runnable alpha shell was admitted
- Big Phase 4 — operability, deployment, failure, and performance proof surfaces were admitted

Active big phase:

- Big Phase 5 — full parity rewrite to production-alpha on the declared Linux lane

Big Phase 5 is the only implementation phase that may change repository status now.
Historical big phases remain reference boundaries only; they are not active work trackers.

## Phase 5 Milestone Gates

Phase 5 advances in this exact order:

1. `Program Reset`
2. `CDC Contract Foundation`
3. `Host and Runtime Foundation`
4. `CLI Foundation`
5. `Command Family Closure`
6. `Proof Closure`
7. `Performance Architecture Overhaul`

Promotion rule:

- do not advance a milestone in substance until its roadmap exit evidence is real
- update parity ledgers and `STATUS.md` when milestone truth changes
- keep deferred, non-lane, and intentional-divergence rows explicit in the ledgers and roadmap index

## Current Gate

Current milestone:

- `CDC Contract Foundation`

Next milestone:

- `Host and Runtime Foundation`

Current gate requires:

- baseline-backed closure of the lane-blocking CDC rows called out in `STATUS.md`
- roadmap-index ownership and evidence references that match the CDC ledger
- no reliance on deleted Phase 5 planning/status files or stage-named evidence surfaces

## Production-Alpha Gate

Production-alpha is not claimed at `Proof Closure`.

Production-alpha readiness requires all of the following after `Performance Architecture Overhaul`:

- no parity regressions across the admitted lane
- architecture boundaries still satisfy the crate-dependency contract in `STATUS.md`
- performance-critical hot paths are rerun with the admitted probes or benchmarks
- deployment, failure/recovery, and parity evidence reruns are green

## Canonical Sources

- tracked status: `STATUS.md`
- roadmap: `docs/phase-5/roadmap.md`
- exact row ownership: `docs/phase-5/roadmap-index.csv`
- parity evidence: `docs/parity/README.md` and the domain ledgers
