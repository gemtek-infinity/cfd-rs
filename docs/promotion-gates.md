# Promotion Gates

This document defines the current promotion model for the rewrite.
It stays compact on purpose: [`STATUS.md`](../STATUS.md) owns tracked status, and
[`docs/phase-5/roadmap.md`](phase-5/roadmap.md) owns the implementation roadmap.

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
7. `Parity Revalidation`
8. `Performance Architecture Overhaul`

Promotion rule:

- do not advance a milestone in substance until its roadmap exit evidence is real
- update parity ledgers and [`STATUS.md`](../STATUS.md) when milestone truth changes
- keep deferred, non-lane, and intentional-divergence rows explicit in the ledgers and roadmap index

## Current Gate

Current milestone:

- `Parity Revalidation`

Next milestone:

- `Performance Architecture Overhaul`

Current gate requires:

- full re-audit pass across all 150 rows with current source-grounded evidence
- HIS-069 ICMP proxy runtime validation
- every intentional divergence has an explicit deferred boundary and rationale
- no stale or historical-only parity claims remain

Previous milestone (`Proof Closure`) is complete.

## Production-Alpha Gate

Production-alpha is not claimed at `Parity Revalidation`.

Production-alpha readiness requires all of the following after `Parity Revalidation` and `Performance Architecture Overhaul`:

- no parity regressions across the admitted lane
- architecture boundaries still satisfy the crate-dependency contract in [`STATUS.md`](../STATUS.md)
- performance-critical hot paths are rerun with the admitted probes or benchmarks
- logging compatibility reruns are green across CLI, local sinks, journald/systemd, and upstream management flows
- deployment, failure/recovery, and parity evidence reruns are green

## Canonical Sources

- tracked status: [`STATUS.md`](../STATUS.md)
- roadmap: [`docs/phase-5/roadmap.md`](phase-5/roadmap.md)
- exact row ownership: [`docs/phase-5/roadmap-index.csv`](phase-5/roadmap-index.csv)
- parity evidence: [`docs/parity/README.md`](parity/README.md) and the domain ledgers
